use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::process::exit;
use std::sync::Arc;
use bytes::Bytes;
use async_nats::jetstream::Context;
use log::debug;
use tokio::sync::Mutex;
use tw_econ::Econ;
use liquid::{ParserBuilder, Template};
use regex::{Captures, Regex};
use crate::model::{Env, EnvHandler, RegexModel};

pub fn read_yaml_file(file_path: &str) -> Result<Env, Box<dyn Error>> {
    let mut file = File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let env: Env = serde_yaml::from_str(&contents)?;
    Ok(env)
}

pub async fn econ_connect(env: Env) -> std::io::Result<Arc<Mutex<Econ>>> {
    let econ = Arc::new(Mutex::new(Econ::new()));

    let econ_clone = econ.clone();
    let mut econ_lock = econ_clone.lock().await;
    econ_lock.connect(env.get_econ_addr())?;

    if let Some(msg) = env.auth_message {
        econ_lock.set_auth_message(msg);
    }

    if env.econ_password.is_none() {
        panic!("econ_password must be set");
    }

    let authed = econ_lock.try_auth(env.econ_password.unwrap())?;
    if !authed {
        panic!("Econ client is not authorized");
    }

    Ok(econ)
}

pub fn template(template: &str) -> Template {
    if !template.is_empty() {
        debug!("template: {}", template);
    }

    ParserBuilder::with_stdlib()
        .build()
        .expect("Template error created")
        .parse(template)
        .expect("Template error parsed")
}

fn format_mention(nickname: String) -> String {
    if nickname.is_empty() {
        return nickname;
    }

    if nickname.contains('@') && nickname.len() > 2 {
        return nickname.replace("@", "@-");
    }
    nickname
}

pub fn generate_text(reg: Captures, pattern: &RegexModel, env: &EnvHandler) -> Option<(String, String)> {
    if reg.len() == 3 {
        return Some((
            format_mention(reg.get(1)?.as_str().to_string()),
            reg.get(2)?.as_str().to_string())
        );
    }


    let obj = liquid::object!({
        "text_leave": &env.text_leave,
        "text_join": &env.text_join
    });

    let obj_text = liquid::object!({
        "player": reg.get(1)?.as_str().to_string(),
        "text": pattern.template.render(&obj).expect("Template render error, generate_text")
    });

    let formatted_text = format_mention(
        env.text.render(&obj_text).unwrap()
    );
    Some((String::new(), formatted_text))

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
            &caps.get(1).expect("Format_regex except").as_str().to_string(),
            &t,
            1
        );
    }
    text
}


pub async fn send_message(json: &str, publish_stream: &str, jetstream: &Context) -> Result<(), Box<dyn Error>> {
    jetstream.publish(publish_stream.to_string(), Bytes::from(json.to_owned())).await?;
    Ok(())
}



pub fn err_to_string_and_exit(msg: &str, err: Box<dyn Error>) {
    let text = match err.to_string().as_ref() {
        "Broken pipe (os error 32)" => {"Server closed socket(Broken pipe, os error 32)".to_string()}
        _ => {err.to_string()}
    };
    eprintln!("{}{}", msg, text);
    exit(0);
}

