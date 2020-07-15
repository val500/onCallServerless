use chrono::Utc;
use dynomite::dynamodb::DynamoDbClient;
use futures::{future::Future, try_join, TryFutureExt};
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
use rusoto_core::{Region, RusotoError, RusotoError::Service};
use rusoto_dynamodb::{PutItemError, PutItemOutput};
use rusoto_sqs::{
    GetQueueUrlRequest, SendMessageError,
    SendMessageError::{InvalidMessageContents, UnsupportedOperation},
    SendMessageRequest, SendMessageResult, Sqs, SqsClient,
};
use serde_json::Value;
use simple_logger::init_with_level;
use std::{env, pin::Pin};
use uuid::Uuid;

fn main() {
    init_with_level(Info).unwrap();
    lambda!(handler);
}

fn handler(request: Request, _context: Context) -> Result<Response<Body>, HandlerError> {
    let request_body: Value = serde_urlencoded::from_str(match request.body() {
        Text(string) => string.as_ref(),
        _ => "",
    })
    .unwrap();
    match request.method() {
        &Method::GET => {
            let mut twiml = r#"<?xml version="1.0" encoding="UTF-8"?> <Response> <Say> Please leave a message at the beep </Say> <Record playBeep="true" /> </Response>"#.into_response();
            twiml.headers_mut().insert(
                CONTENT_TYPE,
                HeaderValue::from_str("application/xml").unwrap(),
            );
            Ok(twiml)
        }
        &Method::POST => {
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
            .unwrap_or_else(|| vec![]);

            let call: Call = Call {
                call_id: Uuid::new_v4(),
                message_url: request_body["RecordingUrl"].as_str().unwrap().to_string(),
                phone_number: phone_number.clone(),
                users,
                handled: false,
            };
            let dynamo_client = DynamoDbClient::new(Region::UsEast1);
            let call_future = call.async_write_call(&dynamo_client, call_table);
            let sqs_client = SqsClient::new(Region::UsEast1);
            let sqs_future = sqs_push(&sqs_client, call.call_id.to_string(), phone_number, 20);
            join_futures(call_future, sqs_future)?;
            Ok("Success!".into_response())
        }
        _ => Ok("Invalid Request".into_response()),
    }
}

#[tokio::main]
async fn sqs_push<'a>(
    sqs_client: &'a SqsClient,
    call_id: String,
    group_id: String,
    delay_time: i64,
) -> Pin<Box<dyn Future<Output = Result<SendMessageResult, RusotoError<SendMessageError>>> + 'a>> {
    sqs_client.send_message(SendMessageRequest {
        delay_seconds: Some(delay_time),
        message_body: call_id.to_string(),
        queue_url: sqs_client
            .get_queue_url(GetQueueUrlRequest {
                queue_name: group_id,
                queue_owner_aws_account_id: None,
            })
            .await
            .unwrap()
            .queue_url
            .unwrap(),
        ..SendMessageRequest::default()
    })
}

#[tokio::main]
async fn join_futures<'a>(
    call_future: Pin<
        Box<dyn Future<Output = Result<PutItemOutput, RusotoError<PutItemError>>> + 'a>,
    >,
    sqs_future: Pin<
        Box<dyn Future<Output = Result<SendMessageResult, RusotoError<SendMessageError>>> + 'a>,
    >,
) -> Result<(), HandlerError> {
    try_join!(
        call_future.map_err(|e| { HandlerError::from("CallWriteFail") }),
        sqs_future.map_err(|e| {
            let mut string = String::new();
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
    Ok(())
}
