use crate::range::{ClosedRange, OpenRange, Range};
use crate::time::TimeOfDayDuration;
use crate::users::User;
use chrono::{offset::FixedOffset, DateTime, NaiveDateTime};
use dynomite::{
    dynamodb::{
        AttributeValue, DynamoDb, DynamoDbClient, GetItemInput, PutItemError, PutItemInput,
    },
    error::AttributeError,
    Attribute, FromAttributes, Item,
};
use rusoto_core::{Region, RusotoError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::iter::Iterator;

#[derive(Debug, Clone)]
pub struct ScheduleSlot {
    interval: OpenRange<NaiveDateTime>,
    restriction: TimeOfDayDuration,
    providers: Vec<User>,
}

impl ScheduleSlot {
    pub fn new_schedule_slot(
        interval: OpenRange<NaiveDateTime>,
        restriction: TimeOfDayDuration,
        providers: Vec<User>,
    ) -> ScheduleSlot {
        ScheduleSlot {
            interval,
            restriction,
            providers,
        }
    }
    fn to_iter<'a>(&'a self, offset: FixedOffset) -> Box<dyn Iterator<Item = Entry> + 'a> {
        let start = self.interval.start;
        let end_option = self.interval.end;
        match end_option {
            Some(end) => {
                let zoned_end: DateTime<FixedOffset> =
                    DateTime::<FixedOffset>::from_utc(end, offset);
                let iter = self
                    .restriction
                    .to_iter(&start, offset)
                    .take_while(move |range| range.start < zoned_end)
                    .map(move |range| {
                        let ze = zoned_end;
                        if range.contains(Some(&ze)) {
                            Entry {
                                range: ClosedRange {
                                    start: range.start,
                                    end: ze,
                                },
                                providers: self.providers.clone(),
                            }
                        } else {
                            Entry {
                                range,
                                providers: self.providers.clone(),
                            }
                        }
                    });
                Box::new(iter)
            }
            None => {
                let self_clone = self.clone();
                let iter = self
                    .restriction
                    .to_iter(&start, offset)
                    .map(move |range| Entry {
                        range,
                        providers: self_clone.providers.clone(),
                    });
                Box::new(iter)
            }
        }
    }
}

pub fn generate_schedule(
    slots: Vec<ScheduleSlot>,
    start: NaiveDateTime,
    end: NaiveDateTime,
    offset: FixedOffset,
    group_id: String,
) -> Schedule {
    let schedule_range = OpenRange::new_open_range(&start, &Some(end));
    let entries: Vec<Vec<Entry>> = slots
        .iter()
        .filter(|slot| slot.interval.intersection(&schedule_range) != None)
        .map(|slot| {
            let mut new_slot = slot.clone();
            new_slot.interval = new_slot.interval.intersection(&schedule_range).unwrap();
            new_slot.to_iter(offset).collect::<Vec<_>>()
        })
        .collect();
    Schedule {
        group_id,
        entries: merge_entries(entries),
    }
}

fn merge_entries(entries: Vec<Vec<Entry>>) -> Vec<Entry> {
    let mut new_entries: Vec<Entry> = Vec::new();
    for vec_entry in entries {
        for entry in vec_entry {
            new_entries = merge_into(&new_entries, entry);
        }
    }
    new_entries
}

fn merge_into(entries: &[Entry], entry: Entry) -> Vec<Entry> {
    let overlapped_index = entries.iter().position(|e| e.range.overlaps(&entry.range));
    match overlapped_index {
        Some(o) => {
            let overlapped = entries[o].clone();
            let mut new_entries: Vec<Entry> = entries.to_vec();
            new_entries.remove(o);
            let mut bounds = vec![
                entry.range.start,
                entry.range.end,
                overlapped.range.start,
                overlapped.range.end,
            ];
            bounds.sort();
            if entry.range.start != overlapped.range.start {
                new_entries = merge_into(
                    &new_entries,
                    Entry {
                        range: ClosedRange::new_closed_range(&bounds[0], &bounds[1]),
                        providers: if entry.range.start < overlapped.range.start {
                            entry.providers.to_vec()
                        } else {
                            overlapped.providers.to_vec()
                        },
                    },
                );
            }

            new_entries = merge_into(
                &new_entries,
                Entry {
                    range: ClosedRange::new_closed_range(&bounds[1], &bounds[2]),
                    providers: {
                        let mut new_entry = entry.providers.to_vec();
                        new_entry.append(&mut overlapped.providers.to_vec());
                        new_entry
                    },
                },
            );

            if entry.range.end != overlapped.range.end {
                new_entries = merge_into(
                    &new_entries,
                    Entry {
                        range: ClosedRange::new_closed_range(&bounds[2], &bounds[3]),
                        providers: if entry.range.end > overlapped.range.end {
                            entry.providers.to_vec()
                        } else {
                            overlapped.providers.to_vec()
                        },
                    },
                )
            }
            new_entries
        }
        None => {
            let mut new_entries = entries.to_vec();
            new_entries.push(entry);
            new_entries
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Entry {
    range: ClosedRange<DateTime<FixedOffset>>,
    providers: Vec<User>,
}

impl Attribute for Entry {
    fn into_attr(self) -> AttributeValue {
        AttributeValue {
            l: Some(vec![self.range.into_attr(), self.providers.into_attr()]),
            ..AttributeValue::default()
        }
    }
    fn from_attr(value: AttributeValue) -> Result<Self, AttributeError> {
        match value.l {
            Some(l) => Ok(Entry {
                range: ClosedRange::<DateTime<FixedOffset>>::from_attr(l[0].clone())?,
                providers: Vec::<User>::from_attr(l[1].clone())?,
            }),
            None => Err(AttributeError::InvalidType),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Item, Clone)]
pub struct Schedule {
    #[dynomite(partition_key)]
    group_id: String,
    entries: Vec<Entry>,
}

impl Schedule {
    #[tokio::main]
    pub async fn write_schedule(
        &self,
        table_name: String,
        region: Region,
    ) -> Result<(), RusotoError<PutItemError>> {
        let client = DynamoDbClient::new(region);
        client
            .put_item(PutItemInput {
                table_name,
                item: self.clone().into(), // <= convert schedule into it's attribute map representation
                ..PutItemInput::default()
            })
            .await?;
        Ok(())
    }

    #[tokio::main]
    pub async fn get_schedule(
        table_name: String,
        region: Region,
        key: String,
        primary_key_name: String,
    ) -> Option<Schedule> {
        let client = DynamoDbClient::new(region);
        let mut key_map = HashMap::new();
        key_map.insert(primary_key_name, key.into_attr());
        client
            .get_item(GetItemInput {
                table_name,
                key: key_map,
                ..GetItemInput::default()
            })
            .await
            .ok()
            .map_or_else(|| None, |output| output.item)
            .map_or_else(|| None, |attrs| Schedule::from_attrs(attrs).ok())
    }

    pub fn get_providers(&self, date_time: DateTime<FixedOffset>) -> Option<Vec<User>> {
        self.entries
            .iter()
            .find(|entry| entry.range.contains(Some(&date_time)))
            .map_or_else(|| None, |entry| Some(entry.providers.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::range::OpenRange;
    use crate::time::{TimeOfDay, TimeOfDayDuration};
    use crate::users::User;
    use chrono::{FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Weekday};
    use rusoto_core::Region;
    use NaiveTime::from_hms;

    #[test]
    fn test_schedule() {
        let tobias = User::new_user(
            "1".to_owned(),
            "+12183957949".to_owned(),
            "Tobias Funke".to_owned(),
            "+19149543303".to_owned(),
        );
        let jeff = User::new_user(
            "2".to_owned(),
            "+12183957949".to_owned(),
            "Jeff Winger".to_owned(),
            "+19147251309".to_owned(),
        );
        let test_guy = User::new_user(
            "3".to_owned(),
            "+12183957949".to_owned(),
            "Test Guy".to_owned(),
            "+19143745558".to_owned(),
        );
        let test_guy2 = User::new_user(
            "4".to_owned(),
            "+12183957949".to_owned(),
            "Test Guy2".to_owned(),
            "+13473513315".to_owned(),
        );
        let june1 = NaiveDate::from_ymd(2020, 6, 1).and_hms(0, 0, 0);
        let june30 = NaiveDate::from_ymd(2020, 6, 7).and_hms(0, 0, 0);
        let june_range = OpenRange::new_open_range(&june1, &Some(june30));

        let everyday9 = TimeOfDay::new_tod(from_hms(9, 0, 0), None);
        let everyday5 = TimeOfDay::new_tod(from_hms(17, 0, 0), None);
        let everyday9to5 = TimeOfDayDuration::new_todd(everyday9, everyday5.clone()); //Everyday 9-5
        let slot1 = ScheduleSlot::new_schedule_slot(june_range.clone(), everyday9to5, vec![jeff]);

        let mon12 = TimeOfDay::new_tod(from_hms(12, 0, 0), Some(Weekday::Mon));
        let mon10 = TimeOfDay::new_tod(from_hms(22, 0, 0), Some(Weekday::Mon));
        let mon1210 = TimeOfDayDuration::new_todd(mon12, mon10); //Mon 12-10pm
        let slot2 = ScheduleSlot::new_schedule_slot(june_range.clone(), mon1210, vec![tobias]);

        let everyday10 = TimeOfDay::new_tod(from_hms(22, 0, 0), None);
        let everyday510 = TimeOfDayDuration::new_todd(everyday5, everyday10); //Everyday 5-10
        let slot3 = ScheduleSlot::new_schedule_slot(
            june_range.clone(),
            everyday510,
            vec![test_guy, test_guy2.clone()],
        );

        let tue9 = TimeOfDay::new_tod(from_hms(9, 0, 0), Some(Weekday::Tue));
        let tue5 = TimeOfDay::new_tod(from_hms(17, 0, 0), Some(Weekday::Tue));
        let tue9to5 = TimeOfDayDuration::new_todd(tue9, tue5); //Tue 9-5
        let slot4 = ScheduleSlot::new_schedule_slot(june_range.clone(), tue9to5, vec![test_guy2]);
        let est = FixedOffset::east(-4 * 3600);

        let schedule = generate_schedule(
            vec![slot1, slot2, slot3, slot4],
            june1,
            june30,
            est,
            "+12183957949".to_owned(),
        );
        assert!(!schedule
            .write_schedule(
                "GroupTable".into(),
                Region::Custom {
                    name: "local-stack-1".into(),
                    endpoint: "http://localhost:4566/".into(),
                },
            )
            .is_err());
    }
}
