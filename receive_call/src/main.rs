use chrono::Utc;
use dynomite::dynamodb::DynamoDbClient;
use futures::{try_join, TryFutureExt};
use lambda_http::{
    http::{
        header::{HeaderValue, CONTENT_TYPE},
        method::Method,
    },
    lambda, Body,
    Body::Text,
    IntoResponse, Request, Response,
};
use lambda_runtime::{error::HandlerError, Context};
use log::Level::Info;
use models::{call::Call, schedule::Schedule, users::User};
use rusoto_core::{Region, RusotoError::Service};
use rusoto_sqs::{
    SendMessageError::{InvalidMessageContents, UnsupportedOperation},
    SqsClient,
};
use serde_json::Value;
use simple_logger::init_with_level;
use std::env;
use uuid::Uuid;

fn main() {
    init_with_level(Info).unwrap();
    lambda!(handler);
}

#[tokio::main]
async fn handler(request: Request, _context: Context) -> Result<Response<Body>, HandlerError> {
    let request_body: Value = serde_urlencoded::from_str(match request.body() {
        Text(string) => string.as_ref(),
        _ => "",
    })
    .unwrap();
    match *request.method() {
        Method::GET => {
            let mut twiml = r#"<?xml version="1.0" encoding="UTF-8"?> <Response> <Say> Please leave a message at the beep </Say> <Record playBeep="true" /> </Response>"#.into_response();
            twiml.headers_mut().insert(
                CONTENT_TYPE,
                HeaderValue::from_str("application/xml").unwrap(),
            );
            Ok(twiml)
        }
        Method::POST => {
            let phone_number: String = request_body["To"].as_str().unwrap().to_string();
            let call_table = env::var("CALL_TABLE")?;
            let group_table = env::var("GROUP_TABLE")?;

            let users: Vec<User> = Schedule::get_schedule(
                group_table,
                Region::UsEast1,
                phone_number.clone(),
                "group_id".to_string(),
            )
            .unwrap()
            .get_providers(Utc::now().into())
            .unwrap_or_else(Vec::new);

            let call: Call = Call {
                call_id: Uuid::new_v4(),
                group_id: phone_number.clone(),
                message_url: request_body["RecordingUrl"].as_str().unwrap().to_string(),
                phone_number,
                users,
                handled: false,
            };
            let dynamo_client = DynamoDbClient::new(Region::UsEast1);
            let call_future = call.async_write_call(&dynamo_client, call_table);
            
            let sqs_client = SqsClient::new(Region::UsEast1);
            let sqs_future = call.sqs_push(&sqs_client, 20);
            
            try_join!(
                call_future.map_err(|e| { HandlerError::from("CallWriteFail") }),
                sqs_future.map_err(|e| {
                    let string;
                    HandlerError::from(match e {
                        Service(InvalidMessageContents(s)) => {
                            string = format!("InvalidMessageContents:{}", s);
                            string.as_str()
                        }
                        Service(UnsupportedOperation(s)) => {
                            string = format!("UnsupportedOperation:{}", s);
                            string.as_str()
                        }
                        _ => "OtherErrorFound",
                    })
                })
            )?;
            Ok("Success!".into_response())
        }
        _ => Ok("Invalid Request".into_response()),
    }
}
