mod handlers;
pub mod model;

use crate::econ::model::MsgBridge;
use crate::handler::handlers::chat_handler;
use crate::handler::model::{ConfigHandler, HandlerPaths};
use crate::model::ServerMessageData;
use async_nats::jetstream::Context;
use async_nats::Client;
use futures::future::join_all;
use futures::StreamExt;
use log::{debug, error, info};
use std::process::exit;
use tokio::io;

async fn handler(
    config: ConfigHandler,
    nats: Client,
    jetstream: Context,
    path: HandlerPaths,
    task_count: usize,
) -> Result<(), async_nats::Error> {
    let mut subscriber = nats
        .queue_subscribe(path.from.clone(), format!("handler_{}", task_count))
        .await?;

    info!(
        "Handler started from {} to {:?}, regex.len: {}, job_id: {}",
        path.from,
        path.to,
        path.regex.len(),
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
        for regex in &path.regex {
            if let Some(caps) = regex.captures(&msg.text) {
                let json = chat_handler(&msg, &config, caps, &path).await;

                if json.is_empty() {
                    break;
                }

                let data = ServerMessageData::get_server_name_and_server_name(&msg.args);

                debug!("sent json to {:?}: {}", path.to, json);
                for write_path in &path.to {
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

pub async fn main(config: ConfigHandler, nats: Client, jetstream: Context) -> io::Result<()> {
    let mut tasks = vec![];

    for (task_count, path) in config.clone().paths.into_iter().enumerate() {
        let task = tokio::spawn(handler(
            config.clone(),
            nats.clone(),
            jetstream.clone(),
            path,
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
