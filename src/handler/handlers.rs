use crate::econ::model::MsgBridge;
use crate::handler::model::{ConfigHandler, HandlerPaths, MsgHandler};
use crate::util::emojis::replace_from_emoji;
use crate::util::utils::{format_regex, format_text, generate_text};
use log::error;
use regex::Captures;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::process::exit;

async fn get_json(value: Vec<Option<String>>, yaml_args: YamlValue) -> String {
    let args: JsonValue = serde_json::to_value(yaml_args).unwrap_or_else(|err| {
        error!("Transfer YamlValue to JsonValue Failed: {}", err);
        exit(1)
    });
    let send_msg = MsgHandler { value, args };

    match serde_json::to_string_pretty(&send_msg) {
        Ok(str) => str,
        Err(err) => {
            error!("Json Serialize Error: {}", err);
            exit(1);
        }
    }
}

pub async fn chat_handler(
    msg: &MsgBridge,
    env: &ConfigHandler,
    caps: Captures<'_>,
    pattern: &HandlerPaths,
) -> String {
    if pattern.custom {
        let value: Vec<Option<String>> = caps
            .iter()
            .map(|opt_match| opt_match.map(|m| m.as_str().to_string()))
            .collect();

        return get_json(value, msg.clone().args).await;
    }

    let Some((name, text)) = generate_text(caps, pattern, env) else {
        return String::default();
    };

    let text = format_regex(
        format_text(replace_from_emoji(text), env.block_text_in_chat.clone()),
        env.chat_regex.clone(),
    );

    let name = format_regex(
        format_text(name, env.block_text_in_nickname.clone()),
        env.nickname_regex.clone(),
    );

    let value: Vec<Option<String>> = vec![Some(name), Some(text)];

    get_json(value, msg.clone().args.into()).await
}
