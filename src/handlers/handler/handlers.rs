use crate::handlers::handler::model::MsgHandler;
use log::error;
use regex::Captures;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::process::exit;

async fn get_json(value: Vec<Option<String>>, text: String, yaml_args: &YamlValue) -> String {
    let args: JsonValue = serde_json::to_value(yaml_args).unwrap_or_else(|err| {
        error!("Transfer YamlValue to JsonValue Failed: {}", err);
        exit(1)
    });
    let send_msg = MsgHandler { value, text, args };

    match serde_json::to_string_pretty(&send_msg) {
        Ok(str) => str,
        Err(err) => {
            error!("Json Serialize Error: {}", err);
            exit(1);
        }
    }
}

pub async fn chat_handler(caps: Captures<'_>, args: &YamlValue) -> String {
    let value: Vec<Option<String>> = caps
        .iter()
        .map(|opt_match| opt_match.map(|m| m.as_str().to_string()))
        .collect();

    let (first, rest) = value.split_at(1);
    let first_element = first.first().cloned().flatten().unwrap_or_default();
    let rest_value = rest.to_vec();

    get_json(rest_value, first_element, args).await
}
