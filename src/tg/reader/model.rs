use serde::{Deserialize, Serialize};
use serde_yaml::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct MsgHandler {
    pub value: Vec<String>,
    pub args: Value,
}
