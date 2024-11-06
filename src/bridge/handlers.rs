use std::process::exit;
use std::time::Duration;
use async_nats::jetstream::Context;
use async_nats::jetstream::context::PublishError;
use bytes::Bytes;
use futures::Stream;
use futures_util::StreamExt;
use log::{debug, error};
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;
use tw_econ::Econ;
use crate::model::MsgBridge;

pub async fn process_messages<T>(tx: Sender<String>, mut subscriber: T)
where
    T: Stream<Item = async_nats::Message> + Unpin,
{
    while let Some(message) = subscriber.next().await {
        debug!("Message received from {}, length {}", message.subject, message.length);
        let msg = match std::str::from_utf8(&message.payload) {
            Ok(s) => s.to_string(),
            Err(err) => {
                error!("Error converting bytes to string: {}", err);
                continue;
            }
        };

        if let Err(err) = tx.send(msg).await {
            error!("tx.send error: {}", err);
        }
    }
}

pub async fn msg_reader(mut econ: Econ, jetstream: Context, message_thread_id: String, server_name: String) -> Result<(), PublishError> {
    let publish_stream = format!("tw.econ.read.{}", &message_thread_id.clone());

    loop {
        let line = match econ.recv_line(true).await {
            Ok(result) => { result }
            Err(err) => {
                error!("err from loop: {}", err);
                exit(1);
            }
        };

        if let Some(message) = line {
            debug!("Recevered line from econ: {}", message);
            let send_msg = MsgBridge {
                server_name: server_name.clone(),
                message_thread_id: message_thread_id.clone(),
                text: message,
            };

            let json = match serde_json::to_string_pretty(&send_msg) {
                Ok(result) => { result }
                Err(err) => {error!("Error converting json to string: {}", err); continue}
            };

            debug!("Sending JSON to tw.econ.read.(id): {}", json);
            jetstream.publish(publish_stream.clone(), Bytes::from(json.to_owned())).await?.await?;
        } else {
            sleep(Duration::from_millis(10)).await;
        }
    }
}

pub async fn check_status(tx: Sender<String>, check_status_econ_sleep: Option<u64>) {
    let check_status_econ_sleep = check_status_econ_sleep.unwrap_or(15);
    loop {
        debug!("check status econ");
        match tx.send("".to_string()).await {
            Ok(_) => {}
            Err(err) => {error!("tx.send error: {}", err)}
        };
        sleep(Duration::from_secs(check_status_econ_sleep)).await;
    }
}
