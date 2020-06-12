extern crate chrono;
use crate::range::{ClosedRange, Range};
use chrono::{
    offset::FixedOffset, DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Weekday, TimeZone,
};
use serde::{Deserialize, Serialize};
use std::iter::{successors, Iterator};

#[derive(Clone, Debug)]
struct TimeOfDay {
    time: NaiveTime,
    day_of_week: Option<Weekday>,
}

fn first_weekday_after(dt: DateTime<FixedOffset>, wd: &Weekday) -> DateTime<FixedOffset> {
    let dt_since = dt.weekday().num_days_from_monday() as i64;
    let wd_since = wd.num_days_from_monday() as i64;
    if dt_since < wd_since {
        dt + Duration::days((wd_since - dt_since) as i64)
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
        let now_date = offset
            .from_local_datetime(&NaiveDateTime::new(now.clone(), self.time.clone()))
            .unwrap();
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
        let zoned_now = offset.from_local_datetime(&now).unwrap();
        let zoned_now_clone = zoned_now.clone();
        let mut start_succ_peek = self.start.to_succ(&now.date(), offset).peekable();
        let start: Option<DateTime<FixedOffset>> = match start_succ_peek.peek() {
            Some(d) => Some(d.clone()),
            None => None,
        };
        let start_clone = start.clone();
        let end_succ =
            self.end
                .to_succ(&now.date(), offset)
                .skip_while(move |x| match start_clone {
                    Some(p) => x < &p,
                    None => true,
                });
        let succ = self
            .start
            .to_succ(&now.date(), offset)
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

#[cfg(test)]
mod tests {
    use chrono::{
        offset::FixedOffset, DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Weekday, TimeZone
    };
    use super::*;
    #[test]
    fn test_time_of_day() {
        let t1 = NaiveTime::from_hms(9, 0, 0); // 9:00AM
        let d1 = NaiveDate::from_ymd(2020, 6, 1); // 6-1-2020
        let dt1 = NaiveDateTime::new(d1, t1); // 6-1-2020, 9am

        let d2 = NaiveDate::from_ymd(2020, 6, 2); // 6-2-2020
        let dt2 = NaiveDateTime::new(d2, t1); // 6-2-2020, 9am

        let d3 = NaiveDate::from_ymd(2020, 6, 3); // 6-3-2020
        let dt3 = NaiveDateTime::new(d3, t1); // 6-3-2020, 9am

        let est = FixedOffset::east(-4 * 3600);
        let nine_daily = TimeOfDay {
            time: t1,
            day_of_week: None,
        };
        assert_eq!(
            nine_daily.to_succ(&d1, &est).take(3).collect::<Vec<_>>(),
            vec![
                est.from_local_datetime(&dt1).unwrap(),
                est.from_local_datetime(&dt2).unwrap(),
                est.from_local_datetime(&dt3).unwrap()
            ]
        );
    }
    #[test]
    fn test_time_of_day_2() {
        let t1 = NaiveTime::from_hms(9, 0, 0); // 9:00AM
        let d1 = NaiveDate::from_ymd(2020, 6, 1); // 6-1-2020 (Monday)
        let dt1 = NaiveDateTime::new(d1, t1); // 6-1-2020, 9am

        let d2 = NaiveDate::from_ymd(2020, 6, 3); // 6-3-2020
        let dt2 = NaiveDateTime::new(d2, t1); // 6-10-2020, 9am

        let d3 = NaiveDate::from_ymd(2020, 6, 10); // 6-10-2020
        let dt3 = NaiveDateTime::new(d3, t1); // 6-10-2020, 9am

        let d4 = NaiveDate::from_ymd(2020, 6, 17); // 6-17-2020
        let dt4 = NaiveDateTime::new(d4, t1); // 6-17-2020, 9am

        let est = FixedOffset::east(-4 * 3600);
        let nine_daily = TimeOfDay {
            time: t1,
            day_of_week: Some(Weekday::Wed),
        };
        assert_eq!(
            nine_daily.to_succ(&d1, &est).take(3).collect::<Vec<_>>(),
            vec![
                est.from_local_datetime(&dt2).unwrap(),
                est.from_local_datetime(&dt3).unwrap(),
                est.from_local_datetime(&dt4).unwrap()
            ]
        );
    }
    #[test]
    fn test_time_of_day_3() {
        let t1 = NaiveTime::from_hms(9, 0, 0); // 9:00AM
        let d1 = NaiveDate::from_ymd(2020, 5, 28); // 5-28-2020 (Thursday)
        let dt1 = NaiveDateTime::new(d1, t1); // 5-28-2020, 9am

        let d2 = NaiveDate::from_ymd(2020, 6, 3); // 6-3-2020
        let dt2 = NaiveDateTime::new(d2, t1); // 6-10-2020, 9am

        let d3 = NaiveDate::from_ymd(2020, 6, 10); // 6-10-2020
        let dt3 = NaiveDateTime::new(d3, t1); // 6-10-2020, 9am

        let d4 = NaiveDate::from_ymd(2020, 6, 17); // 6-17-2020
        let dt4 = NaiveDateTime::new(d4, t1); // 6-17-2020, 9am

        let est = FixedOffset::east(-4 * 3600);
        let nine_daily = TimeOfDay {
            time: t1,
            day_of_week: Some(Weekday::Wed),
        };
        assert_eq!(
            nine_daily.to_succ(&d1, &est).take(3).collect::<Vec<_>>(),
            vec![
                est.from_local_datetime(&dt2).unwrap(),
                est.from_local_datetime(&dt3).unwrap(),
                est.from_local_datetime(&dt4).unwrap()
            ]
        );
    }
    
}
