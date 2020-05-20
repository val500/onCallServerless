#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

pub mod schedule;

extern crate dynomite;
extern crate rusoto_core;
extern crate futures;
extern crate tokio;

use dynomite::{
    attr_map,
    dynamodb::{
        AttributeDefinition, CreateTableInput, DynamoDb, DynamoDbClient, GetItemInput,
        KeySchemaElement, ProvisionedThroughput, PutItemInput, ScanInput,
    },
    retry::Policy,
    DynamoDbExt, FromAttributes, Item, Retries,
};
use futures::{Future, Stream};
use rusoto_core::region::Region as Region;
use tokio::runtime::{Runtime};
use std::env;
use std::error::{Error};
use schedule::{Schedule};
use std::str::FromStr;

#[tokio::main]
pub async fn write_open_schedule(s: &Schedule<String>) -> Result<(), Box<dyn Error>> {
    let client = DynamoDbClient::new(rusoto_core::region::Region::UsEast2);
    let table_name = env::var("TABLE_NAME")?;
    println!(
        "Write Open Schedule result {:#?}",
        client
            .put_item(PutItemInput {
                table_name: table_name.clone(),
                item: s.clone().into(), // <= convert book into it's attribute map representation
                ..PutItemInput::default()
            })
            .await?
    );
    Ok(())
}

