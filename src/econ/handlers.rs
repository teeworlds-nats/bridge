use crate::args::Args;
use crate::econ::model::MsgBridge;
use crate::handler::model::MsgHandler;
use crate::model::CowStr;
use crate::nats::Nats;
use crate::util::convert;
use futures_util::StreamExt;
use log::{debug, error, info, trace, warn};
use serde_yaml::Value;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;
use tw_econ::Econ;

pub async fn process_messages<'a>(
    tx: Sender<String>,
    nats: Nats,
    subscriber_str: CowStr<'a>,
    queue: CowStr<'a>,
) {
    info!("Subscribe to the channel: {subscriber_str}");
    let mut subscriber = nats.subscriber(subscriber_str, queue).await;

    while let Some(message) = subscriber.next().await {
        debug!(
            "Message received from {}, length {}",
            message.subject, message.length
        );
        if let Some(msg) = convert::<MsgHandler>(&message.payload) {
            if Args::get(&msg.args, "econ_divide", false) {
                for result in msg.value {
                    if let Err(err) = tx.send(result).await {
                        error!("tx.send error: {err}");
                    }
                }
            } else {
                let result = msg.value.join(" ");
                if let Err(err) = tx.send(result).await {
                    error!("tx.send error: {err}");
                }
            }
        };
    }
}

pub async fn msg_reader(
    mut econ: Econ,
    nats: Nats,
    nats_path: Vec<CowStr<'static>>,
    args: Value,
) -> anyhow::Result<()> {
    loop {
        let line = match econ.recv_line(true).await {
            Ok(Some(result)) => result,
            Err(err) => {
                error!("Reader: err from loop: {err}");
                sleep(Duration::from_secs(5)).await;
                continue;
            }
            _ => continue,
        };
        trace!("Message received from econ: {line}");
        let send_msg = MsgBridge {
            text: line,
            args: args.clone(),
        };

        let json = match send_msg.json() {
            Ok(result) => result,
            Err(err) => {
                warn!("Error converting json to string: {err}");
                continue;
            }
        };

        trace!("Sending JSON to {nats_path:?}: {json}");
        for patch in nats_path.clone() {
            nats.publish(patch, json.to_owned()).await.ok();
        }
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
