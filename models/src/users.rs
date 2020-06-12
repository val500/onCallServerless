use dynomite::{Item, Attribute, Attributes,
               dynamodb::{AttributeValue},
               error::{AttributeError}};
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct User {
    uuid: String,
    group_id: String,
    name: String,
    number: String,
}

impl Attribute for User {
    fn into_attr(self) -> AttributeValue {
        AttributeValue {
            ss: Some(vec![self.uuid, self.group_id, self.name, self.number]),
            ..AttributeValue::default()
        }
    }

    fn from_attr(value: AttributeValue) -> Result<Self, AttributeError> {
        match value.ss {
            Some(ss) => Ok(User {
                uuid: ss[0].clone(),
                group_id: ss[1].clone(),
                name: ss[2].clone(),
                number: ss[3].clone(),
            }),
            None => Err(AttributeError::InvalidType),
        }
    }
}
