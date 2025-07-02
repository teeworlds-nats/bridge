use crate::model::{BaseConfig, CowStr};
use crate::nats::NatsConfig;
use nestify::nest;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serde_yaml::Value;

nest! {
    #[derive(Default, Clone, Deserialize)]
    pub struct ConfigHandler<'a> {
        logging: Option<String>,
        pub nats: NatsConfig<'a>,

        pub paths: Vec<
            #[derive(Default, Clone, Deserialize)]
            pub struct HandlerPaths<'b> {
                pub from: CowStr<'b>,
                pub regex: Vec<String>,
                pub to: Vec<CowStr<'b>>,
                #[serde(default)]
                pub args: Value,
                #[serde(default = "default_paths_queue")]
                pub queue: CowStr<'b>,
            } ||<'a>>,

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
    
    async fn default_config() ->  &'static str {
        include_str!("../default_config/handler.yaml")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MsgHandler {
    pub text: String,
    pub value: Vec<String>,
    pub args: JsonValue,
}

impl MsgHandler {
    pub async fn get_json(value: Vec<String>, text: String, yaml_args: &Value) -> String {
        let args: JsonValue = serde_json::to_value(yaml_args).unwrap_or_else(|err| {
            panic!("Transfer YamlValue to JsonValue Failed: {err}");
        });
        let send_msg = MsgHandler { value, text, args };

        match serde_json::to_string_pretty(&send_msg) {
            Ok(str) => str,
            Err(err) => {
                panic!("Json Serialize Error: {err}");
            }
        }
    }
}

fn default_paths_queue() -> CowStr<'static> {
    CowStr::Borrowed("handler_{{0}}")
}
