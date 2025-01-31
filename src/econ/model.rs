use serde_derive::{Deserialize, Serialize};
use serde_yaml::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsgBridge {
    pub text: String,
    pub args: Value,
}
