use crate::handler::model::MsgHandler;
use regex::Captures;
use serde_yaml::Value as YamlValue;

pub async fn chat_handler(caps: &Captures<'_>, args: &YamlValue) -> String {
    let value: Vec<String> = caps
        .iter()
        .map(|opt_match| opt_match.map_or_else(|| "".to_string(), |m| m.as_str().to_string()))
        .collect();

    let (first, rest) = value.split_at(1);
    let first_element = first.first().cloned().unwrap_or_default();
    let rest_value = rest.to_vec();

    MsgHandler::get_json(rest_value, first_element, args).await
}
