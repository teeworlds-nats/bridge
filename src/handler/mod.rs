mod handlers;
pub mod model;

use crate::econ::model::MsgBridge;
use crate::handler::handlers::chat_handler;
use crate::model::{Config, NatsHandlerPaths};
use crate::util::{get_and_format_caps, merge_yaml_values};
use async_nats::jetstream::Context;
use async_nats::Client;
use futures::future::join_all;
use futures::StreamExt;
use log::{debug, error, info, trace, warn};
use regex::Regex;
use tokio::io;

async fn handler(
    nats: Client,
    jetstream: Context,
    path: NatsHandlerPaths,
    task_count: usize,
) -> Result<(), async_nats::Error> {
    let re: Vec<Regex> = path
        .regex
        .iter()
        .filter_map(|r| Regex::new(r).ok())
        .collect();
    let args = path.args.unwrap_or_default();

    let sub_path = path
        .queue
        .replace("{{task_count}}", &task_count.to_string());
    info!(
        "Handler started from {} to {:?}, regex.len: {}, job_id: {}, sub_path: {}",
        path.from,
        path.to,
        re.len(),
        task_count,
        sub_path
    );
    let mut subscriber = nats.queue_subscribe(path.from, sub_path.clone()).await?;
    while let Some(message) = subscriber.next().await {
        debug!(
            "message received from {}, length {}, job_id: {}, sub_path: {}",
            message.subject, message.length, task_count, sub_path
        );
        let msg: MsgBridge = match std::str::from_utf8(&message.payload) {
            Ok(json_string) => serde_json::from_str(json_string).unwrap_or_else(|err| {
                panic!("Error deserializing JSON: {err}");
            }),
            Err(err) => {
                warn!("Error converting bytes to string: {err}");
                continue;
            }
        };
        let new_args = merge_yaml_values(&msg.args, &args);

        for regex in &re {
            if let Some(caps) = regex.captures(&msg.text) {
                let json = chat_handler(&caps, &new_args).await;

                let write_paths: Vec<String> = path
                    .to
                    .iter()
                    .map(|x| get_and_format_caps(x, &new_args, Some(&caps)).to_string())
                    .collect();
                trace!("send {json} to {write_paths:?}:");
                for path in write_paths {
                    jetstream.publish(path, json.clone().into()).await?;
                }
            }
        }
    }

    Ok(())
}

pub async fn main<'a>(config: Config<'a>, nats: Client, jetstream: Context) -> io::Result<()> {
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
                error!("Task failed: {e:?}");
                return Err(io::Error::other("One of the tasks failed"));
            }
            Err(e) => {
                error!("Task panicked: {e:?}");
                return Err(io::Error::other("One of the tasks panicked"));
            }
        }
    }

    Ok(())
}
