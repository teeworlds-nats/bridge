use crate::handler::model::MsgHandler;
use regex::Captures;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;

async fn get_json(value: Vec<String>, text: String, yaml_args: &YamlValue) -> String {
    let args: JsonValue = serde_json::to_value(yaml_args).unwrap_or_else(|err| {
        panic!("Transfer YamlValue to JsonValue Failed: {}", err);
    });
    let send_msg = MsgHandler { value, text, args };

    match serde_json::to_string_pretty(&send_msg) {
        Ok(str) => str,
        Err(err) => {
            panic!("Json Serialize Error: {}", err);
        }
    }
}

pub async fn chat_handler(caps: &Captures<'_>, args: &YamlValue) -> String {
    let value: Vec<String> = caps
        .iter()
        .map(|opt_match| opt_match.map_or_else(|| "".to_string(), |m| m.as_str().to_string()))
        .collect();

    let (first, rest) = value.split_at(1);
    let first_element = first.first().cloned().unwrap_or_default();
    let rest_value = rest.to_vec();

    get_json(rest_value, first_element, args).await
}
