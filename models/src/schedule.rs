extern crate chrono;
extern crate dynomite;
use std::str::FromStr;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use std::cmp::Ordering;
use std::vec::Vec;
use serde::{Serialize, Deserialize};
use dynomite::{Item, Attribute, FromAttributes, Attributes,
               dynamodb::{AttributeValue},
               error::{AttributeError}};

#[derive(Clone, Serialize, Deserialize, Debug)]
enum Label {
    Open,
    Closed,
}

#[derive(PartialEq, Eq, Copy, Clone, Serialize, Deserialize, Debug)]
pub struct Range {
    start_time: DateTime<Utc>,
    end_time: Option<DateTime<Utc>>,
}

impl PartialOrd for Range {
    fn partial_cmp(&self, other: &Range) -> Option<Ordering> {
        if self.overlaps(other) {
            None
        } else {
            if self.start_time < other.start_time {
                Some(Ordering::Less)
            } else {
                Some(Ordering::Greater)
            }
        }
    }
}
impl Range {
    fn contains(&self, ot: &Option<DateTime<Utc>>) -> bool {
        match ot {
            Some(t) => 
                match self.end_time {
                    Some(et) => t < &et && t >= &self.start_time,
                    None => t >= &self.start_time,
                }
            None => false
        }
    }

    fn overlaps(&self, r2: &Range) -> bool {
        self.contains(&Some(r2.start_time)) || self.contains(&r2.end_time) || r2.contains(&Some(self.start_time)) || r2.contains(&self.end_time)
    }

}

impl Attribute for Range {
    fn into_attr(self) -> AttributeValue {
        match self.end_time {
            Some(t) =>
                AttributeValue {
                    ss: Some(vec![self.start_time.to_rfc3339(), t.to_rfc3339()]),
                    ..AttributeValue::default()
                },
            None =>
                AttributeValue {
                    s: Some(self.start_time.to_rfc3339()),
                    ..AttributeValue::default()
                }
        }
    }

    fn from_attr(value: AttributeValue) -> Result<Self, AttributeError> {
        match value.ss {
            Some(ss) =>
                Ok(Range {
                    start_time:
                    match DateTime::<Utc>::from_str(&ss[0]) {
                        Ok(t) => t,
                        Err(e) => return Err(AttributeError::InvalidType)
                    },
                    end_time:
                    match DateTime::<Utc>::from_str(&ss[1]) {
                        Ok(t) => Some(t),
                        Err(e) => return Err(AttributeError::InvalidType)
                    },
                }),
            None =>
                match value.s {
                    Some(s) =>
                        Ok(Range {
                            start_time:
                            match DateTime::<Utc>::from_str(&s) {
                                Ok(t) => t,
                                Err(e) => return Err(AttributeError::InvalidType)
                            },
                            end_time: None,
                        }),
                    None => Err(AttributeError::InvalidType)
                }
                
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub struct ScheduleEntry<T: Clone + Attribute> {
    time_range: Range,
    meta_data: T,
}

impl<T: Clone + Attribute> Attribute for ScheduleEntry<T> {
    fn into_attr(self) -> AttributeValue {
        AttributeValue {
            l: Some(vec![self.time_range.into_attr(), self.meta_data.into_attr()]),
            ..AttributeValue::default()
        }
    }

    fn from_attr(value: AttributeValue) -> Result<Self, AttributeError> {
        match value.l {
            Some(v) =>
                Ok(ScheduleEntry {
                    time_range: Range::from_attr(v[0].clone())?,
                    meta_data: T::from_attr(v[1].clone())?,
                }),
            None => Err(AttributeError::InvalidType)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Schedule<T> where T: Clone + Attribute {
    id: String,
    entries: Vec<ScheduleEntry<T>>,
}

impl<T> Schedule<T> where T: Clone + Attribute {
    fn get_entry(&self, time: DateTime<Utc>) -> Option<&ScheduleEntry<T>> {
        for entry in &self.entries {
            if entry.time_range.contains(&Some(time)) {
                return Some(entry)
            }
        }
        return None
    }
    fn insert_entry(&mut self, se: ScheduleEntry<T>) {
        let mut temp = Vec::new();
        let mut inserted = false;
        if se.time_range < self.entries[0].time_range {
            temp.push(se.clone());
            inserted = true;
        }
        for entry in &self.entries {
            if entry.time_range.overlaps(&se.time_range) {
                if !inserted {
                    temp.push(se.clone());
                    inserted = true;
                }
            } else {
                if se.time_range > entry.time_range && !inserted {
                    temp.push(entry.clone());
                    temp.push(se.clone());
                    inserted = true;
                } else {
                    temp.push(entry.clone());
                }
            }
        }
        self.entries = temp;
    }
}

impl Schedule<String> {
    
}

impl<T: Clone + Attribute> Item for Schedule<T> {
    fn key(&self) -> Attributes {
        let mut attrs = HashMap::new();
        attrs.insert("id".into(), self.id.clone().into_attr());
        attrs
    }
}

impl<T: Clone + Attribute> FromAttributes for Schedule<T> {
    fn from_attrs(attrs: Attributes) -> Result<Self, AttributeError> {
        Ok(Self {
            id: attrs
                .get("id")
                .and_then(|val| val.s.clone())
                .ok_or(AttributeError::MissingField { name: "id".into() })?,
            entries: attrs
                .get("entries")
                .and_then(|val| val.l.clone())
                .ok_or(AttributeError::MissingField { name: "entries".into() })?
                .iter().map(|x| ScheduleEntry::from_attr(x.clone()))
                .collect::<Result<Vec<ScheduleEntry<T>>, AttributeError>>()?,
        })
    }
}

impl<T: Clone + Attribute> Into<Attributes> for Schedule<T> {
    fn into(self: Self) -> Attributes {
        let mut attrs = HashMap::new();
        attrs.insert("id".into(), self.id.into_attr());
        attrs.insert("entries".into(), self.entries.into_attr());
        attrs
    }
}
