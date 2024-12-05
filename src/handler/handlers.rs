use crate::model::{EnvHandler, HandlerPaths, MsgBridge, MsgHandler};
use crate::util::emojis::replace_from_emoji;
use crate::util::utils::{format_regex, format_text, generate_text};
use log::error;
use regex::Captures;
use std::process::exit;

async fn get_json(
    server_name: String,
    args: Vec<Option<String>>,
    message_thread_id: String,
) -> String {
    let send_msg = MsgHandler {
        server_name: Some(server_name),
        args,
        message_thread_id,
    };

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
    env: &EnvHandler,
    caps: Captures<'_>,
    pattern: &HandlerPaths,
) -> String {
    if pattern.custom {
        let args: Vec<Option<String>> = caps
            .iter()
            .map(|opt_match| opt_match.map(|m| m.as_str().to_string()))
            .collect();

        return get_json(msg.server_name.clone(), args, msg.message_thread_id.clone()).await;
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

    let args: Vec<Option<String>> = vec![Some(name), Some(text)];

    get_json(msg.server_name.clone(), args, msg.message_thread_id.clone()).await
}
