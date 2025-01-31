use crate::handler::model::{ConfigHandler, HandlerPaths};
use crate::model::Config;
use log::error;
use regex::{Captures, Regex};
use std::error::Error;
use std::process::exit;
use tw_econ::Econ;

pub async fn econ_connect(env: Config) -> std::io::Result<Econ> {
    let mut econ = Econ::new();
    if env.econ.is_none() {
        error!("econ must be set, see config_example.yaml");
        exit(1);
    }
    let econ_env = env.econ.clone().unwrap();

    if econ_env.password.is_none() {
        error!("econ.password must be set");
        exit(1);
    }

    econ.connect(env.get_econ_addr()).await?;
    if let Some(auth_message) = econ_env.auth_message {
        econ.set_auth_message(auth_message)
    }

    let authed = econ.try_auth(econ_env.password.unwrap()).await?;
    if !authed {
        error!("Econ client is not authorized");
        exit(1);
    }

    Ok(econ)
}

pub fn format_mention(nickname: String) -> String {
    if nickname.is_empty() {
        return nickname;
    }

    if nickname.contains('@') && nickname.len() > 2 {
        return nickname.replace("@", "@-");
    }
    nickname
}

pub fn generate_text(
    reg: Captures,
    pattern: &HandlerPaths,
    env: &ConfigHandler,
) -> Option<(String, String)> {
    if reg.len() == 3 {
        return Some((
            format_mention(reg.get(1)?.as_str().to_string()),
            reg.get(2)?.as_str().to_string(),
        ));
    }

    let text = pattern
        .template
        .replacen("{{text_leave}}", &env.text_leave, 1)
        .replacen("{{text_join}}", &env.text_join, 1)
        .replacen("{{text_edit_nickname}}", &env.text_edit_nickname, 1);

    Some((
        String::new(),
        format_mention(env.text.replacen("{{text}}", &text, 1).replacen(
            "{{player}}",
            reg.get(1)?.as_str(),
            1,
        )),
    ))
}

pub fn format_text(mut text: String, text_vec: Vec<(String, String)>) -> String {
    for (r, t) in text_vec {
        text = text.replacen(&r.to_string(), &t, 1);
    }
    text
}

pub fn format_regex(mut text: String, regex_vec: Vec<(Regex, String)>) -> String {
    for (reg, t) in regex_vec {
        if !reg.is_match(&text) {
            continue;
        }
        let caps = reg.captures(&text).unwrap();
        text = text.replacen(
            &caps
                .get(1)
                .expect("Format_regex except")
                .as_str()
                .to_string(),
            &t,
            1,
        );
    }
    text
}

pub fn err_to_string_and_exit(msg: &str, err: Box<dyn Error>) {
    let text = match err.to_string().as_ref() {
        "Broken pipe (os error 32)" => "Server closed socket(Broken pipe, os error 32)".to_string(),
        _ => err.to_string(),
    };
    error!("{}{}", msg, text);
    exit(1);
}

pub async fn replace_value<T>(input: T, message_thread_id: &str, server_name: &str) -> Vec<String>
where
    T: IntoIterator<Item = String>,
{
    input
        .into_iter()
        .map(|item| {
            item.replace("{{message_thread_id}}", message_thread_id)
                .replace("{{server_name}}", server_name)
        })
        .collect()
}

pub async fn replace_value_single(
    value: &str,
    message_thread_id: &str,
    server_name: &str,
) -> String {
    value
        .replace("{{message_thread_id}}", message_thread_id)
        .replace("{{server_name}}", server_name)
}
