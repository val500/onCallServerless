extern crate chrono;
use crate::range::{ClosedRange, Range};
use chrono::{
    offset::FixedOffset, DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Weekday,
};
use serde::{Deserialize, Serialize};
use std::iter::{successors, Iterator};

#[derive(Clone, Debug)]
struct TimeOfDay {
    time: NaiveTime,
    day_of_week: Option<Weekday>,
}

fn first_weekday_after(dt: DateTime<FixedOffset>, wd: &Weekday) -> DateTime<FixedOffset> {
    let dt_since = dt.weekday().num_days_from_monday();
    let wd_since = wd.num_days_from_monday();
    if dt_since < wd_since {
        dt + Duration::days((dt_since - wd_since) as i64)
    } else if dt_since > wd_since {
        dt + Duration::days((7 - (dt_since - wd_since)) as i64)
    } else {
        dt
    }
}

impl TimeOfDay {
    fn to_succ(
        &self,
        now: &NaiveDate,
        offset: &FixedOffset,
    ) -> Box<dyn Iterator<Item = DateTime<FixedOffset>>> {
        let now_date = DateTime::from_utc(
            NaiveDateTime::new(now.clone(), self.time.clone()),
            offset.clone(),
        );
        match self.day_of_week {
            Some(d) => {
                let adj_now = first_weekday_after(now_date, &d);
                let succ = successors(Some(adj_now), |date| Some(date.clone() + Duration::days(7)));
                Box::new(succ)
            }
            None => {
                let succ = successors(Some(now_date), |date| {
                    Some(date.clone() + Duration::days(1))
                });
                Box::new(succ)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct TimeOfDayDuration {
    start: TimeOfDay,
    end: TimeOfDay,
}

impl TimeOfDayDuration {
    pub fn to_iter(
        &self,
        now: &NaiveDateTime,
        offset: &FixedOffset,
    ) -> Box<dyn Iterator<Item = ClosedRange<DateTime<FixedOffset>>>> {
        let zoned_now = DateTime::from_utc(now.clone(), offset.clone());
        let zoned_now_clone = zoned_now.clone();
        let mut start_succ_peek = self.start.to_succ(&now.date(), offset).peekable();
        let start: Option<DateTime<FixedOffset>>  = match start_succ_peek.peek() {
            Some(d) => Some(d.clone()),
            None => None,
        };
        let start_clone = start.clone();
        let end_succ = self
            .end
            .to_succ(&now.date(), offset)
            .skip_while(move |x| match start_clone {
                Some(p) => x < &p,
                None => true,
            });
        let succ = self.start.to_succ(&now.date(), offset)
            .zip(end_succ)
            .map(|(start, end)| ClosedRange { start, end })
            .skip_while(|range| range.start > range.end)
            .map(move |range| {
                if range.contains(Some(&zoned_now)) {
                    ClosedRange {
                        start: zoned_now_clone,
                        end: range.end,
                    }
                } else {
                    range
                }
            });
        Box::new(succ)
    }
}
