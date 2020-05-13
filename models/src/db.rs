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

#[cfg(feature = "default")]
use rusoto_core_default::Region;

#[cfg(feature = "rustls")]
use rusoto_core_rustls::Region;

use tokio::runtime::Runtime;
