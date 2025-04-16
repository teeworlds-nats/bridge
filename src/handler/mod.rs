mod handlers;
pub mod model;

use crate::econ::model::MsgBridge;
use crate::handler::handlers::chat_handler;
use crate::model::{Config, NatsHandlerPaths};
use crate::util::{get_and_format, merge_yaml_values};
use async_nats::jetstream::Context;
use async_nats::Client;
use futures::future::join_all;
use futures::StreamExt;
use log::{debug, error, info};
use regex::Regex;
use tokio::io;

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
                panic!("Error deserializing JSON: {}", err);
            }),
            Err(err) => {
                error!("Error converting bytes to string: {}", err);
                continue;
            }
        };
        for regex in &re {
            if let Some(caps) = regex.captures(&msg.text) {
                let new_args = merge_yaml_values(&msg.args, &args);
                let json = chat_handler(&caps, &new_args).await;

                let write_paths: Vec<String> = to
                    .iter()
                    .map(|x| get_and_format(x, &new_args, Some(&caps)).to_string())
                    .collect();
                debug!("send {} to {:?}:", json, write_paths);
                for path in write_paths {
                    jetstream.publish(path, json.clone().into()).await?;
                }
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
