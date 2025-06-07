mod handlers;
pub mod model;

use crate::econ::model::MsgBridge;
use crate::handler::handlers::chat_handler;
use crate::handler::model::{ConfigHandler, HandlerPaths};
use crate::model::BaseConfig;
use crate::util::{convert, format_single, get_and_format_caps, merge_yaml_values};
use anyhow::Error;
use async_nats::jetstream::Context;
use async_nats::Client;
use futures::future::join_all;
use futures::StreamExt;
use log::{debug, error, info, trace};
use regex::Regex;
use serde_yaml::Value;
use std::borrow::Cow;
use tokio::io;

async fn handler<'a>(
    nats: Client,
    jetstream: Context,
    path: HandlerPaths<'a>,
    main_args: Value,
    task_count: usize,
) -> Result<(), async_nats::Error> {
    let re: Vec<Regex> = path
        .regex
        .iter()
        .filter_map(|r| Regex::new(r).ok())
        .collect();
    let args = merge_yaml_values(&main_args, &path.args.unwrap_or_default());

    let sub_path = format_single(
        path.queue,
        &args,
        &[task_count.to_string()],
        Cow::Owned("handler_{{0}}".to_string()),
    );
    info!(
        "Handler started from {} to {:?}, regex.len: {}, job_id: {}, sub_path: {}",
        path.from,
        path.to,
        re.len(),
        task_count,
        sub_path
    );
    let mut subscriber = nats
        .queue_subscribe(path.from, sub_path.clone().to_string())
        .await?;
    while let Some(message) = subscriber.next().await {
        debug!(
            "message received from {}, length {}, job_id: {}, sub_path: {}",
            message.subject, message.length, task_count, sub_path
        );
        let msg = match convert::<MsgBridge>(&message.payload) {
            Some(msg) => msg,
            None => continue,
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

pub async fn main(config_path: String) -> anyhow::Result<()> {
    let config = ConfigHandler::load_yaml(&config_path).await?;
    config.set_logging();

    let nats = config.connect_nats().await.unwrap();
    let jetstream = async_nats::jetstream::new(nats.clone());

    let args = config.args.clone().unwrap_or_default();
    let mut tasks = vec![];

    for (task_count, path) in config
        .paths
        .expect("config.nats.paths expect")
        .into_iter()
        .enumerate()
    {
        let task = tokio::spawn(handler(
            nats.clone(),
            jetstream.clone(),
            path,
            args.clone(),
            task_count,
        ));
        tasks.push(task);
    }

    let results = join_all(tasks).await;

    for result in results {
        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                error!("Task failed: {e:?}");
                return Err(Error::from(io::Error::other("One of the tasks failed")));
            }
            Err(e) => {
                error!("Task panicked: {e:?}");
                return Err(Error::from(io::Error::other("One of the tasks panicked")));
            }
        }
    }

    Ok(())
}
