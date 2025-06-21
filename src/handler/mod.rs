mod handlers;
pub mod model;

use crate::args::Args;
use crate::econ::model::MsgBridge;
use crate::format::format;
use crate::handler::handlers::chat_handler;
use crate::handler::model::{ConfigHandler, HandlerPaths};
use crate::model::{BaseConfig, CowString};
use crate::nats::Nats;
use crate::util::{captures_to_list, convert};
use anyhow::Error;
use futures::future::join_all;
use futures::StreamExt;
use log::{debug, error, info, trace};
use regex::Regex;
use serde_yaml::Value;
use tokio::io;
use crate::format_values;

async fn handler<'a>(
    nats: Nats,
    path: HandlerPaths<'a>,
    main_args: Value,
    task_count: usize,
) -> Result<(), async_nats::Error> {
    let re: Vec<Regex> = path
        .regex
        .iter()
        .filter_map(|r| Regex::new(r).ok())
        .collect();
    let args = Args::merge_yaml_values(&main_args, &path.args);

    let sub_path = format_values!(
        path.queue,
        &args,
        &[task_count.to_string()];
        single
    );
    info!(
        "Handler started from {} to {:?}, regex.len: {}, job_id: {}, sub_path: {}",
        path.from,
        path.to,
        re.len(),
        task_count,
        sub_path
    );
    let mut subscriber = nats.subscriber(path.from, sub_path.clone()).await;
    while let Some(message) = subscriber.next().await {
        debug!(
            "message received from {}, length {}, job_id: {}, sub_path: {}",
            message.subject, message.length, task_count, sub_path
        );
        let msg = match convert::<MsgBridge>(&message.payload) {
            Some(msg) => msg,
            None => continue,
        };
        let new_args = Args::merge_yaml_values(&msg.args, &args);

        for regex in &re {
            if let Some(caps) = regex.captures(&msg.text) {
                let json = chat_handler(&caps, &new_args).await;

                let write_paths: Vec<CowString<'a>> = format::format(
                    path.to.clone(),
                    &new_args,
                    &captures_to_list(&caps),
                    Vec::new(),
                );
                trace!("send {json} to {write_paths:?}:");
                for path in write_paths {
                    nats.publish(path, json.to_owned()).await.ok();
                }
            }
        }
    }

    Ok(())
}

pub async fn main(config_path: String) -> anyhow::Result<()> {
    let config = ConfigHandler::load_yaml(&config_path).await?;
    config.set_logging();

    let nats = config.connect_nats().await?;
    let args = config.args.clone().unwrap_or_default();
    let mut tasks = vec![];

    for (task_count, path) in config.paths.into_iter().enumerate() {
        let task = tokio::spawn(handler(nats.clone(), path, args.clone(), task_count));
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
