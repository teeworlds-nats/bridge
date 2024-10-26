use std::net::IpAddr;
use std::str::FromStr;
use regex::Captures;
use crate::model::RconData;

pub async fn process_rcon(caps: Captures<'_>, commands_sync: &[String], commands_log: &[String]) -> RconData {
    let rcon = caps.get(1).unwrap().as_str();

    if rcon.is_empty() {
        return RconData::default();
    }

    let mut words = rcon.split_whitespace();

    let mut sync: bool = false;
    let mut log: bool = false;

    if let Some(first_word) = words.next() {
        if commands_sync.contains(&first_word.to_string()) { sync = true }
        if commands_log.contains(&first_word.to_string()) { log = true }
    }

    if let Some(second_word) = words.next() {
        return if IpAddr::from_str(second_word).is_ok() {
            RconData {
                text: Some(rcon.to_string()),
                sync,
                log
            }
        } else {
            RconData::default()
        };
    }
    RconData::default()
}