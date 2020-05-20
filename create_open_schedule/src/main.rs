extern crate models;
mod lambda_gateway;
use lambda_runtime::{error::HandlerError, lambda, Context};
use models::schedule::{Schedule, ScheduleEntry, JsonSchedule};
use serde::{Serialize, Deserialize};
use lambda_gateway::{LambdaRequest, LambdaResponse, LambdaResponseBuilder};

fn main() {
  lambda!(handler);
}

#[derive(Serialize, Clone)]
struct CustomOutput {
    message: String,
}


fn handler(e: LambdaRequest<JsonSchedule<String>>, ctx: Context) -> Result<LambdaResponse, HandlerError> {
    let json_schedule: &JsonSchedule<String> = e.body();
    let schedule = json_schedule.create_schedule(e
                                                 .request_context
                                                 .identity
                                                 .cognito_identity_id
                                                 .clone());
    match models::write_open_schedule(&schedule) {
        Ok(_) =>
            Ok(
                LambdaResponseBuilder::new()
                    .with_status(200)
                    .with_json(CustomOutput {
                        message: String::from("Successfully Written Schedule!"),
                    })
                    .build()
            ),
        Err(e) =>
            Ok(
                LambdaResponseBuilder::new()
                    .with_status(403)
                    .with_json(CustomOutput {
                        message: format!("{:?}", e),
                    })
                    .build()
            )
    }
}

