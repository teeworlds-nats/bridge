use crate::model::Config;
use async_nats::jetstream::Context;
use async_nats::Client;
use futures_util::future::join_all;
use futures_util::StreamExt;
use log::{debug, error, info};
use serde_json::Value;
use std::process::exit;
use anyhow::{anyhow, Error};
use tokio::io;
use crate::util::utils::get_value_from_path;

fn format_string_with_values(template: &Value, data: &Value) -> Result<String, anyhow::Error> {
    let template_str = template.as_str().ok_or_else(|| {
        Err(anyhow!("Template is not a string"))
    })?;
    let mut result = template_str.to_string();

    for key in data.as_object().unwrap().keys() {
        let placeholder = format!("{{{}}}", key);
        if result.contains(&placeholder) {
            let value = data[key].to_string();
            result = result.replace(&placeholder, &value);
        }
    }

    for key in data.as_object().unwrap().keys() {
        if let Some(nested) = data[key].as_object() {
            for nested_key in nested.keys() {
                let placeholder = format!("{{{}.{} }}", key, nested_key);
                if result.contains(&placeholder) {
                    let value = nested[nested_key].to_string();
                    result = result.replace(&placeholder, &value);
                }
            }
        }
    }

    Ok(result)
}

pub async fn main(config: Config, nats: Client, jetstream: Context) -> io::Result<()> {
    let from = config
        .nats
        .clone()
        .from
        .unwrap_or(vec!["tw.econ.read.*".to_string()]);
    let to = config.nats.clone().to.unwrap_or(vec![
        "tw.{{regex_name}}.{{logging_level}}.{{logging_name}}".to_string(),
    ]);

    if to.len() > 1 || from.len() > 1 {
        error!("count nats.from or nats.to > 1");
        exit(1)
    }

    let from = from.first().unwrap().to_owned();
    let to = to.first().unwrap().to_owned();

    let mut subscriber = nats.subscribe(from.clone()).await.expect("Failed subscribe");

    info!("Handler started from {} to {}", from, to);
    while let Some(message) = subscriber.next().await {
        debug!(
            "message received from {}, length {}",
            message.subject, message.length
        );
        let data: Value = match serde_json::from_slice(&message.payload) {
            Ok(json_string) => serde_json::from_str(json_string).unwrap_or_else(|err| {
                error!("Error deserializing JSON: {}", err);
                exit(1);
            }),
            Err(err) => {
                error!("Error converting bytes to string: {}", err);
                exit(1);
            }
        };
        match format_string_with_values() {
            Ok(_) => {}
            Err(_) => {}
        }

        let json = serde_json::to_string(&data).unwrap();
        debug!("sent json to {}: {}", path, json);
        jetstream
            .publish(path, json.clone().into())
            .await
            .expect("Error publish message to tw.messages");
    }

    Ok(())
}
