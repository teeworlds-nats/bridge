use crate::model::{BaseConfig, CowString, NatsConfig};
use nestify::nest;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serde_yaml::Value;

nest! {
    #[derive(Default, Clone, Deserialize)]
    pub struct ConfigHandler<'a> {
        logging: Option<String>,
        pub nats: NatsConfig<'a>,

        pub paths: Option<Vec<
            #[derive(Default, Clone, Deserialize)]
            pub struct HandlerPaths<'b> {
                pub from: String,
                pub regex: Vec<String>,
                pub to: Vec<String>,
                pub args: Option<Value>,
                pub queue: Option<CowString<'b>>,
            } ||<'a>>>,

        pub args: Option<Value>,
    }
}

impl<'a> BaseConfig for ConfigHandler<'a> {
    fn nats_config(&self) -> &NatsConfig<'_> {
        &self.nats
    }

    fn logging_config(&self) -> Option<String> {
        self.logging.clone()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MsgHandler {
    pub text: String,
    pub value: Vec<String>,
    pub args: JsonValue,
}
