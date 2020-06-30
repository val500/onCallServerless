use dynomite::{dynamodb::AttributeValue, error::AttributeError, Attribute};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct User {
    uuid: String,
    group_id: String,
    name: String,
    pub number: String,
}
impl User {
    pub fn new_user(uuid: String, group_id: String, name: String, number: String) -> User {
        User {
            uuid,
            group_id,
            name,
            number,
        }
    }
}
impl Attribute for User {
    fn into_attr(self) -> AttributeValue {
        let mut map = HashMap::new();
        map.insert("uuid".to_string(), self.uuid.into_attr());
        map.insert("group_id".to_string(), self.group_id.into_attr());
        map.insert("name".to_string(), self.name.into_attr());
        map.insert("number".to_string(), self.number.into_attr());
        AttributeValue {
            m: Some(map),
            ..AttributeValue::default()
        }
    }

    fn from_attr(value: AttributeValue) -> Result<Self, AttributeError> {
        match value.m {
            Some(m) => Ok(User {
                uuid: String::from_attr(m.get(&"uuid".to_string()).unwrap().clone())?,
                group_id: String::from_attr(m.get(&"group_id".to_string()).unwrap().clone())?,
                name: String::from_attr(m.get(&"name".to_string()).unwrap().clone())?,
                number: String::from_attr(m.get(&"number".to_string()).unwrap().clone())?,
            }),
            None => Err(AttributeError::InvalidType),
        }
    }
}
