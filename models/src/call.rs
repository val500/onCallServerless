use crate::users::User;
use dynomite::{
    dynamodb::{DynamoDb, DynamoDbClient, GetItemInput, PutItemError, PutItemInput, PutItemOutput},
    Item, FromAttributes, Attribute
};
use futures::Future;
use rusoto_core::{Region, RusotoError};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, pin::Pin};
use uuid::Uuid;
use rusoto_sqs::{
    GetQueueUrlRequest, SendMessageError,
    SendMessageRequest, SendMessageResult, Sqs, SqsClient,
};


#[derive(Serialize, Deserialize, Debug, Clone, Item)]
pub struct Call {
    #[dynomite(partition_key)]
    pub call_id: Uuid,
    pub group_id: String,
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

    #[tokio::main]
    pub async fn get_call(table_name: String, region: Region, key: String) -> Option<Call> {
        let client = DynamoDbClient::new(region);
        let mut key_map = HashMap::new();
        key_map.insert("call_id".to_string(), key.into_attr());
        client
            .get_item(GetItemInput {
                table_name,
                key: key_map,
                ..GetItemInput::default()
            })
            .await
            .ok()
            .map_or_else(|| None, |output| output.item)
            .map_or_else(|| None, |attrs| Call::from_attrs(attrs).ok())
        
    }

    #[tokio::main]
    pub async fn sqs_push<'a>(
        &self,
        sqs_client: &'a SqsClient,
        delay_time: i64,
    ) -> Pin<Box<dyn Future<Output = Result<SendMessageResult, RusotoError<SendMessageError>>> + 'a>> {
        let call_id = self.call_id;
        let group_id = self.group_id.clone();
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
}
