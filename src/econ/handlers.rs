use crate::econ::model::{ConnectionState, MsgBridge};
use crate::handler::model::MsgHandler;
use crate::model::CowString;
use crate::util::convert;
use async_nats::jetstream::context::PublishError;
use async_nats::jetstream::Context;
use async_nats::subject::ToSubject;
use async_nats::Client;
use bytes::Bytes;
use futures_util::StreamExt;
use log::{debug, error, info, trace, warn};
use serde_yaml::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;
use tw_econ::Econ;

pub async fn process_pending_messages(state: Arc<ConnectionState>, tx: Sender<String>) {
    loop {
        let messages = {
            let mut pending = state.pending_messages.lock().await;
            if pending.is_empty() {
                None
            } else {
                let messages = pending.drain(..).collect::<Vec<_>>();
                Some(messages)
            }
        };

        if let Some(messages) = messages {
            if let Err(err) = tx.send(messages.join("\n")).await {
                error!("tx.send error: {err}");
            }
        }

        sleep(Duration::from_secs(10)).await;
    }
}

pub async fn process_messages<'a>(
    tx: Sender<String>,
    subscriber_str: CowString<'a>,
    queue: CowString<'a>,
    nats: Client,
) {
    let mut subscriber = if queue.is_empty() {
        nats.subscribe(subscriber_str.to_subject()).await
    } else {
        nats.queue_subscribe(subscriber_str.to_subject(), queue.to_string())
            .await
    }
    .unwrap_or_else(|err| {
        error!("Failed to subscribe to {subscriber_str}: {err}");
        std::process::exit(1);
    });

    info!("Subscribe to the channel: {subscriber_str}");
    while let Some(message) = subscriber.next().await {
        debug!(
            "Message received from {}, length {}",
            message.subject, message.length
        );
        let msg = match convert::<MsgHandler>(&message.payload) {
            Some(msg) => msg,
            None => continue,
        };
        let result = msg.value.join(" ");
        if let Err(err) = tx.send(result).await {
            error!("tx.send error: {err}");
        }
    }
}

pub async fn msg_reader(
    mut econ: Econ,
    jetstream: Context,
    nats_path: Vec<CowString<'static>>,
    args: Value,
) -> Result<(), PublishError> {
    loop {
        let line = match econ.recv_line(true).await {
            Ok(result) => result,
            Err(err) => {
                error!("err from loop: {err}");
                break;
            }
        };

        if let Some(message) = line {
            debug!("Recevered line from econ: {message}");
            let send_msg = MsgBridge {
                text: message,
                args: args.clone(),
            };

            let json = match serde_json::to_string_pretty(&send_msg) {
                Ok(result) => result,
                Err(err) => {
                    warn!("Error converting json to string: {err}");
                    continue;
                }
            };

            trace!("Sending JSON to {nats_path:?}: {json}");
            for send_path in nats_path.clone() {
                jetstream
                    .publish(send_path.into_owned(), Bytes::from(json.to_owned()))
                    .await?
                    .await?;
            }
        }
    }
    panic!("msg_reader dead");
}

pub async fn check_status(tx: Sender<String>, check_message: String, check_status_econ_sec: u64) {
    loop {
        trace!("check status econ, msg: \"{check_message}\" sleep: {check_status_econ_sec}",);
        tx.send(check_message.clone())
            .await
            .expect("tx.send error, check_status failed");
        sleep(Duration::from_secs(check_status_econ_sec)).await;
    }
}

pub async fn task(tx: Sender<String>, command: String, sleep_sec: u64) {
    loop {
        debug!("tasks: send message to econ, msg: \"{command}\" sleep: {sleep_sec}",);
        tx.send(command.clone())
            .await
            .expect("tx.send error, task failed");
        sleep(Duration::from_secs(sleep_sec)).await;
    }
}
