use aws_lambda_events::event::sqs::SqsEvent;
use lambda_runtime::{error::HandlerError, lambda, Context};
use log::Level::Info;
use models::call::Call;
use rusoto_core::{Region, RusotoError::Service};
use rusoto_sqs::{
    SendMessageError::{InvalidMessageContents, UnsupportedOperation},
    SqsClient,
};
use simple_logger::init_with_level;
use std::env;
fn main() {
    init_with_level(Info).unwrap();
    lambda!(handler);
}

#[tokio::main]
async fn handler(sqs_event: SqsEvent, _context: Context) -> Result<String, HandlerError> {
    let message: String = sqs_event.records[0].body.as_ref().unwrap().to_string();
    let call_table: String = env::var("CALL_TABLE")?;
    let call = Call::get_call(call_table, Region::UsEast1, message)
        .ok_or_else(|| HandlerError::from("Call Not Found"))?;
    if call.handled {
        Ok("Call Handled!".to_string())
    } else {
        let sqs_client = SqsClient::new(Region::UsEast1);
        let delay_time = 50; // Get from escalation rules
        // TODO: Notify user
        call.sqs_push(&sqs_client, delay_time).await.map_err(|e| {
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
        })?;
        Ok("Renotified user!".to_string())
    }
}
