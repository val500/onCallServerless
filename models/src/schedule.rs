extern crate chrono;
use crate::range::{ClosedRange, OpenRange, Range};
use crate::time::TimeOfDayDuration;
use crate::users::User;
use chrono::{offset::FixedOffset, DateTime, NaiveDateTime};
use serde::{Deserialize, Serialize};
use std::iter::Iterator;
use dynomite::{Item, Attribute, Attributes,
               dynamodb::{AttributeValue},
               error::{AttributeError}};

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

#[derive(Debug, Clone)]
pub struct ScheduleSlot {
    interval: OpenRange<NaiveDateTime>,
    restriction: TimeOfDayDuration,
    providers: Vec<User>,
}

impl ScheduleSlot {
    fn to_iter<'a>(&'a self, offset: &'a FixedOffset) -> Box<dyn Iterator<Item = Entry> + 'a> {
        let start = self.interval.start;
        let end_option = self.interval.end;
        match end_option {
            Some(end) => {
                let zoned_end: DateTime<FixedOffset> =
                    DateTime::<FixedOffset>::from_utc(end.clone(), *offset);
                let clone_1 = zoned_end.clone();
                let clone_2 = zoned_end.clone();
                let iter = self
                    .restriction
                    .to_iter(&start, offset)
                    .take_while(move |range| range.start < clone_1)
                    .map(move |range| {
                        let ze = clone_2;
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
            new_slot.to_iter(&offset).collect::<Vec<_>>()
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

fn merge_into(entries: &Vec<Entry>, entry: Entry) -> Vec<Entry> {
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
#[derive(Serialize, Deserialize, Debug, Item)]
pub struct Schedule {
    #[dynomite(partition_key)]
    group_id: String,
    entries: Vec<Entry>,
}
