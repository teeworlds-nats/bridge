use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use std::option::Option;

#[derive(Debug, Serialize, Deserialize)]
pub struct MsgHandler {
    pub text: String,
    pub value: Vec<Option<String>>,
    pub args: Value,
}
