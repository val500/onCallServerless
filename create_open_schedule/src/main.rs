extern crate models;
mod lambda_gateway;
use lambda_runtime::{error::HandlerError, lambda, Context};
use models::schedule::{Schedule, ScheduleEntry};
use serde::{Serialize, Deserialize};
use lambda_gateway::{LambdaRequest, LambdaResponse, LambdaResponseBuilder};

fn main() {
  lambda!(handler);
}

#[derive(Serialize, Clone)]
struct CustomOutput {
    message: String,
}


fn handler(e: LambdaRequest<Schedule<String>>, ctx: Context) -> Result<LambdaResponse, HandlerError> {
    let schedule = e.body();
    let response = LambdaResponseBuilder::new()
        .with_status(200)
        .with_json(CustomOutput {
            message: format!("{:?}", schedule),
        })
        .build();
    Ok(response)
}

