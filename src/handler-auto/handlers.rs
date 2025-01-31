use crate::econ::model::MsgBridge;
use crate::handler_auto::model::{Data, MsgHandlerAuto};
use chrono::NaiveDateTime;
use log::error;
use regex::Captures;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::process::exit;

async fn get_data(data: Data, yaml_args: YamlValue) -> MsgHandlerAuto {
    let args: JsonValue = serde_json::to_value(yaml_args).unwrap_or_else(|err| {
        error!("Transfer YamlValue to JsonValue Failed: {}", err);
        exit(1)
    });
    MsgHandlerAuto { data, args }
}

fn get_caps_date(caps: &Captures, index: usize) -> Option<i64> {
    if let Some(matched) = caps.get(index) {
        let date_str = matched.as_str();
        NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S")
            .map(|naive_datetime| naive_datetime.and_utc().timestamp())
            .ok()
    } else {
        None
    }
}

fn get_caps(caps: &Captures, index: usize) -> String {
    caps.get(index)
        .expect("Expected a match for the given index")
        .as_str()
        .to_string()
}

pub async fn caps_handler(msg: &MsgBridge, caps: Captures<'_>) -> MsgHandlerAuto {
    let mut timestamp: Option<i64> = None;
    let mut logging_level = String::new();
    let mut logging_name = String::new();
    let mut text = String::new();

    let caps_len = caps.len();

    if caps_len == 5 {
        timestamp = get_caps_date(&caps, 1);
        logging_level = get_caps(&caps, 2);
        logging_name = get_caps(&caps, 3);
        text = get_caps(&caps, 4);
    } else if caps_len == 3 {
        logging_level = "INK".to_string();
        logging_name = get_caps(&caps, 1);
        text = get_caps(&caps, 2);
    }

    let data = Data {
        timestamp,
        text,
        logging_level,
        logging_name,
    };

    get_data(data, msg.clone().args.into()).await
}
