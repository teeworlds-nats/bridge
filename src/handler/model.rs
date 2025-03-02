use crate::model::{Config, NatsHandlerPaths};
use nestify::nest;
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::option::Option;

#[derive(Debug, Serialize, Deserialize)]
pub struct MsgHandler {
    pub value: Vec<Option<String>>,
    pub args: Value,
}

nest! {
    #[derive(Debug, Clone)]
    pub struct ConfigHandler {
        pub paths: Vec<
            #[derive(Debug, Clone)]
            pub struct HandlerPaths {
                pub from: String,
                pub regex: Vec<Regex>,
                pub to: Vec<String>,
                pub template: String,
                pub custom: bool,
            }>,
        pub text: String,
        pub text_leave: String,
        pub text_join: String,
        pub text_edit_nickname: String,
        pub nickname_regex: Vec<(Regex, String)>,
        pub block_text_in_nickname: Vec<(String, String)>,
        pub chat_regex: Vec<(Regex, String)>,
        pub block_text_in_chat: Vec<(String, String)>,
    }
}
impl From<NatsHandlerPaths> for HandlerPaths {
    fn from(item: NatsHandlerPaths) -> Self {
        let from = item.from.unwrap();
        let regex = item
            .regex
            .unwrap_or_default()
            .iter()
            .map(|x| Regex::new(x).unwrap())
            .collect();
        let to = item.to.unwrap_or_default();
        let template = item.template.unwrap_or_default();
        let custom = item.custom.unwrap_or_default();

        HandlerPaths {
            from,
            regex,
            to,
            template,
            custom,
        }
    }
}

impl Config {
    pub fn get_env_handler(&self) -> Result<ConfigHandler, Box<dyn Error>> {
        let handler_paths: Vec<HandlerPaths> = self
            .nats
            .paths
            .clone()
            .unwrap_or_default()
            .iter()
            .map(|path| HandlerPaths::from(path.clone())) // Преобразование каждого элемента
            .collect();

        Ok(ConfigHandler {
            paths: handler_paths,
            text: self
                .text
                .clone()
                .unwrap_or_else(|| "{{player}} {{text}}".to_string()),
            text_leave: self
                .text_leave
                .clone()
                .unwrap_or_else(|| "has left the game".to_string()),
            text_join: self
                .text_join
                .clone()
                .unwrap_or_else(|| "has join the game".to_string()),
            text_edit_nickname: self
                .text_edit_nickname
                .clone()
                .unwrap_or_else(|| "{{player}} > {{text}}".to_string()),
            nickname_regex: self
                .nickname_regex
                .clone()
                .unwrap_or_default()
                .iter()
                .filter_map(|(k, v)| {
                    Regex::new(k).ok().map(|regex| (regex, v.clone()))
                })
                .collect(),
            block_text_in_nickname: self
                .block_text_in_nickname
                .clone()
                .unwrap_or_else(|| {
                    vec![
                        ("tw/".to_string(), "".to_string()),
                        ("twitch.tv/".to_string(), "".to_string()),
                    ]
                })
                .into_iter()
                .collect(),
            chat_regex: self
                .chat_regex
                .clone()
                .unwrap_or_default()
                .iter()
                .filter_map(|(k, v)| {
                    Regex::new(k).ok().map(|regex| (regex, v.clone()))
                })
                .collect(),
            block_text_in_chat: self
                .block_text_in_chat
                .clone()
                .unwrap_or_default()
                .into_iter()
                .collect(),
        })
    }
}
