use crate::users::User;
use dynomite::{
    dynamodb::{DynamoDb, DynamoDbClient, PutItemError, PutItemInput, PutItemOutput},
    Item,
};
use futures::Future;
use rusoto_core::RusotoError;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use uuid::Uuid;
#[derive(Serialize, Deserialize, Debug, Clone, Item)]
pub struct Call {
    #[dynomite(partition_key)]
    pub call_id: Uuid,
    pub message_url: String,
    pub phone_number: String,
    pub users: Vec<User>,
    pub handled: bool,
}

impl Call {
    #[tokio::main]
    pub async fn async_write_call<'a>(
        &self,
        client: &'a DynamoDbClient,
        table_name: String,
    ) -> Pin<Box<dyn Future<Output = Result<PutItemOutput, RusotoError<PutItemError>>> + 'a>> {
        client.put_item(PutItemInput {
            table_name,
            item: self.clone().into(), // <= convert schedule into it's attribute map representation
            ..PutItemInput::default()
        })
    }
}
