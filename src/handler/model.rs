use serde_derive::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct MsgHandler {
    pub text: String,
    pub value: Vec<String>,
    pub args: Value,
}
