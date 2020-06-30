use chrono::{FixedOffset, NaiveDate, NaiveTime, Weekday};
use lambda_http::{lambda, IntoResponse, Request};
use lambda_runtime::{error::HandlerError, Context};
use models::range::OpenRange;
use models::schedule::{generate_schedule, ScheduleSlot};
use models::time::{TimeOfDay, TimeOfDayDuration};
use models::users::User;
use rusoto_core::Region;
use std::env;

fn main() {
    lambda!(handler);
}

fn handler(_request: Request, _context: Context) -> Result<impl IntoResponse, HandlerError> {
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
    let june30 = NaiveDate::from_ymd(2020, 7, 30).and_hms(0, 0, 0);
    let june_range = OpenRange::new_open_range(&june1, &Some(june30));

    let everyday9 = TimeOfDay::new_tod(NaiveTime::from_hms(9, 0, 0), None);
    let everyday5 = TimeOfDay::new_tod(NaiveTime::from_hms(17, 0, 0), None);
    let everyday9to5 = TimeOfDayDuration::new_todd(everyday9, everyday5.clone()); //Everyday 9-5
    let slot1 = ScheduleSlot::new_schedule_slot(june_range.clone(), everyday9to5, vec![jeff]);

    let mon12 = TimeOfDay::new_tod(NaiveTime::from_hms(12, 0, 0), Some(Weekday::Mon));
    let mon10 = TimeOfDay::new_tod(NaiveTime::from_hms(22, 0, 0), Some(Weekday::Mon));
    let mon1210 = TimeOfDayDuration::new_todd(mon12, mon10); //Mon 12-10pm
    let slot2 = ScheduleSlot::new_schedule_slot(june_range.clone(), mon1210, vec![tobias]);

    let everyday10 = TimeOfDay::new_tod(NaiveTime::from_hms(22, 0, 0), None);
    let everyday510 = TimeOfDayDuration::new_todd(everyday5, everyday10); //Everyday 5-10
    let slot3 = ScheduleSlot::new_schedule_slot(
        june_range.clone(),
        everyday510,
        vec![test_guy, test_guy2.clone()],
    );

    let tue9 = TimeOfDay::new_tod(NaiveTime::from_hms(9, 0, 0), Some(Weekday::Tue));
    let tue5 = TimeOfDay::new_tod(NaiveTime::from_hms(17, 0, 0), Some(Weekday::Tue));
    let tue9to5 = TimeOfDayDuration::new_todd(tue9, tue5); //Tue 9-5
    let slot4 = ScheduleSlot::new_schedule_slot(june_range, tue9to5, vec![test_guy2]);
    let est = FixedOffset::east(-4 * 3600);

    let schedule = generate_schedule(
        vec![slot1, slot2, slot3, slot4],
        june1,
        june30,
        est,
        "+12183957949".to_owned(),
    );
    let table_name = env::var("TABLE_NAME").unwrap();
    let region = Region::UsEast1;
    schedule
        .write_schedule(table_name, region)
        .map_err(|_e| HandlerError::from("Write Fail"))?;
    Ok("Written Successfully!")
}
