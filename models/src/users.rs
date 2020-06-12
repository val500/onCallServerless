use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct User {
    uuid: String,
    group_id: String,
    name: String,
    number: String,
}

