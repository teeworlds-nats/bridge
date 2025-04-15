mod handlers;
pub mod model;
mod regex;

use crate::econ::model::MsgBridge;
use crate::handlers::handler_auto::handlers::caps_handler;
use crate::handlers::handler_auto::regex::DEFAULT_REGEX;
use crate::model::{Config, RegexData};
use async_nats::jetstream::Context;
use async_nats::Client;
use futures_util::future::join_all;
use futures_util::StreamExt;
use log::{debug, error, info};
use tokio::io;

async fn handler(
    config: Config,
    nats: Client,
    jetstream: Context,
    regex: &RegexData,
    task_count: usize,
) -> Result<(), async_nats::Error> {
    let from = config
        .nats
        .clone()
        .from
        .unwrap_or(vec!["tw.econ.read.*".to_string()]);
    let to = config.nats.clone().to.unwrap_or(vec![
        "tw.{{regex_name}}.{{logging_level}}.{{logging_name}}".to_string(),
    ]);

    if to.len() > 1 || from.len() > 1 {
        panic!("count nats.from or nats.to > 1");
    }

    let from = from.first().unwrap().to_owned();
    let to = to.first().unwrap().to_owned();

    let mut subscriber = nats
        .queue_subscribe(from.clone(), format!("handler_auto_{}", task_count))
        .await?;

    info!(
        "Handler started from {} to {}, regex_name: {}, job_id: {}",
        from, to, regex.name, task_count
    );
    while let Some(message) = subscriber.next().await {
        debug!(
            "message received from {}, length {}, job_id: {}",
            message.subject, message.length, task_count
        );
        let msg: MsgBridge = match std::str::from_utf8(&message.payload) {
            Ok(json_string) => serde_json::from_str(json_string).unwrap_or_else(|err| {
                panic!("Error deserializing JSON: {}", err);
            }),
            Err(err) => {
                error!("Error converting bytes to string: {}", err);
                continue
            }
        };
        if let Some(caps) = regex.regex.captures(&msg.text) {
            let data = caps_handler(&msg, caps).await;
            let json = match serde_json::to_string_pretty(&data) {
                Ok(str) => str,
                Err(err) => {
                    error!("Json Serialize Error: {}", err);
                    continue
                }
            };
            if data.data.text.is_empty() {
                break;
            }

            let path = to
                .replace("{{regex_name}}", &regex.name)
                .replace("{{logging_level}}", &data.data.logging_level)
                .replace("{{logging_name}}", &data.data.logging_name);

            debug!("sent json to {}: {}", path, json);
            jetstream
                .publish(path, json.clone().into())
                .await
                .expect("Error publish message to tw.messages");
        }
    }

    Ok(())
}

pub async fn main(config: Config, nats: Client, jetstream: Context) -> io::Result<()> {
    let mut tasks = vec![];

    for (task_count, regex) in DEFAULT_REGEX.iter().enumerate() {
        let task = tokio::spawn(handler(
            config.clone(),
            nats.clone(),
            jetstream.clone(),
            regex,
            task_count,
        ));
        tasks.push(task);
    }

    let results = join_all(tasks).await;

    for result in results {
        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                error!("Task failed: {:?}", e);
                return Err(io::Error::other("One of the tasks failed"));
            }
            Err(e) => {
                error!("Task panicked: {:?}", e);
                return Err(io::Error::other("One of the tasks panicked"));
            }
        }
    }

    Ok(())
}
