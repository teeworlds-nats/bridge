mod handlers;
pub mod model;

use crate::econ::model::MsgBridge;
use crate::handlers::handler::handlers::chat_handler;
use crate::model::{Config, NatsHandlerPaths, ServerMessageData};
use async_nats::jetstream::Context;
use async_nats::Client;
use futures::future::join_all;
use futures::StreamExt;
use log::{debug, error, info};
use regex::Regex;
use serde_yaml::Value;
use std::process::exit;
use tokio::io;

fn merge_yaml_values(original: &Value, new: &Value) -> Value {
    match (original, new) {
        (Value::Mapping(original_map), Value::Mapping(new_map)) => {
            let mut merged_map = original_map.clone();

            for (key, new_value) in new_map {
                merged_map.insert(key.clone(), new_value.clone());
            }

            Value::Mapping(merged_map)
        }
        _ => original.clone(),
    }
}

async fn handler(
    nats: Client,
    jetstream: Context,
    path: NatsHandlerPaths,
    task_count: usize,
) -> Result<(), async_nats::Error> {
    let from = path.from.expect("nats.paths[?].from except");
    let to = path.to.expect("nats.paths[?].from except");
    let re: Vec<Regex> = path
        .regex
        .clone()
        .expect("nats.paths[?].regex expected")
        .iter()
        .filter_map(|r| Regex::new(r).ok())
        .collect();
    let args = path.args.unwrap_or_default();

    let mut subscriber = nats
        .queue_subscribe(from.clone(), format!("handler_{}", task_count))
        .await?;

    info!(
        "Handler started from {} to {:?}, regex.len: {}, job_id: {}",
        from,
        to,
        re.len(),
        task_count
    );
    while let Some(message) = subscriber.next().await {
        debug!(
            "message received from {}, length {}, job_id: {}",
            message.subject, message.length, task_count
        );
        let msg: MsgBridge = match std::str::from_utf8(&message.payload) {
            Ok(json_string) => serde_json::from_str(json_string).unwrap_or_else(|err| {
                error!("Error deserializing JSON: {}", err);
                exit(1);
            }),
            Err(err) => {
                error!("Error converting bytes to string: {}", err);
                exit(1);
            }
        };
        for regex in &re {
            if let Some(caps) = regex.captures(&msg.text) {
                let new_args = merge_yaml_values(&msg.args, &args);
                println!("{:?}", new_args);
                let json = chat_handler(caps, &new_args).await;

                if json.is_empty() {
                    break;
                }

                let data = ServerMessageData::get_server_name_and_server_name(&new_args);

                debug!("sent json to {:?}: {}", to, json);
                for write_path in &to {
                    let path = data.replace_value_single(write_path).await;

                    jetstream
                        .publish(path, json.clone().into())
                        .await
                        .expect("Error publish message to tw.messages");
                }
                break;
            }
        }
    }

    Ok(())
}

pub async fn main(config: Config, nats: Client, jetstream: Context) -> io::Result<()> {
    let mut tasks = vec![];

    for (task_count, path) in config
        .nats
        .clone()
        .paths
        .expect("config.nats.paths expect")
        .into_iter()
        .enumerate()
    {
        let task = tokio::spawn(handler(nats.clone(), jetstream.clone(), path, task_count));
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
