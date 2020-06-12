extern crate models;
mod lambda_gateway;
use lambda_runtime::{error::HandlerError, lambda, Context};
use serde::{Serialize, Deserialize};
use lambda_gateway::{LambdaRequest, LambdaResponse, LambdaResponseBuilder};

fn main() {

}

#[derive(Serialize, Clone)]
struct CustomOutput {
    message: String,
}

