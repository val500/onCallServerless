use chrono::Utc;
use chrono::{DateTime, FixedOffset};
use lambda_http::{lambda, Body, Body::Text, IntoResponse, Request, RequestExt, Response, http::header::{HeaderValue, CONTENT_TYPE}};
use lambda_runtime::{error::HandlerError, Context};
use log::Level::Info;
use models::schedule::Schedule;
use serde_json::Value;
use serde_urlencoded::from_str;
use simple_logger::init_with_level;
use std::env;
fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    lambda!(handler);
}

fn handler(request: Request, _context: Context) -> Result<Response<Body>, HandlerError> {
    println!("{:?}", request);

    let request_body: Value = serde_urlencoded::from_str(match request.body() {
        Text(string) => string.as_ref(),
        _ => "",
    })
    .unwrap();
    println!("{:?}", request_body);
    let phone_number: String = request_body["To"].as_str().unwrap().to_string();
    let table_name = env::var("TABLE_NAME")?;

    let schedule = Schedule::get_schedule(
        table_name,
        rusoto_core::region::Region::UsEast1,
        phone_number,
        "group_id".to_string(),
    )
    .unwrap();
    println!(
        "{:?}",
        call_numbers(
            schedule
                .get_providers(Utc::now().into())
                .unwrap_or_else(|| vec![])
                .iter()
                .map(|user| &user.number)
                .collect(),
        )
    );
    println!(
        "Twilio: {:?}, Providers: {:?}, Time: {:?}",
        call_numbers(
            schedule
                .get_providers(Utc::now().into())
                .unwrap_or_else(|| vec![])
                .iter()
                .map(|user| &user.number)
                .collect(),
        ),
        schedule.get_providers(Utc::now().into()),
        DateTime::<FixedOffset>::from(Utc::now())
    );
    let mut twiml = call_numbers(
        schedule
            .get_providers(Utc::now().into())
            .unwrap_or_else(|| vec![])
            .iter()
            .map(|user| &user.number)
            .collect(),
    ).into_response();
    twiml.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_str("application/xml").unwrap());
    Ok(twiml)
    
}

fn call_numbers(numbers: Vec<&String>) -> String {
    let mut twiml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Response>
    <Dial>
"#.to_owned();
    for num in numbers {
        twiml.push_str(format!("        <Number>{}</Number>\n", num).as_ref())
    }
    twiml.push_str("    </Dial> \n </Response>");
    twiml
}
