mod handlers;
pub mod model;

use crate::econ::handlers::{check_status, msg_reader, process_messages, task};
use crate::econ::model::ConfigEcon;
use crate::model::{BaseConfig, CowString, MsgError};
use crate::util::{format, format_single};
use async_nats::subject::ToSubject;
use bytes::Bytes;
use log::{error, info, warn};
use std::borrow::Cow;
use tokio::sync::mpsc;

pub async fn main(config_path: String) -> anyhow::Result<()> {
    let config = ConfigEcon::load_yaml(&config_path).await?;
    config.set_logging();

    let nats = config.connect_nats().await.unwrap();
    let jetstream = async_nats::jetstream::new(nats.clone());

    let (tx, mut rx) = mpsc::channel(64);
    let econ_reader = config
        .econ_connect()
        .await
        .expect("econ_reader failed connect");
    let mut econ_write = config
        .econ_connect()
        .await
        .expect("econ_write failed connect");
    info!("econ connected");

    let conf_nats = config.nats.clone();
    let args = config.args.clone().unwrap_or_default();

    let read_path: Vec<CowString> = format(
        conf_nats.from,
        &args,
        &[],
        vec![
            Cow::Owned("tw.econ.write.{{message_thread_id}}".to_string()),
            Cow::Owned("tw.econ.moderator".to_string()),
        ],
    );
    let write_path: Vec<CowString> = format(
        conf_nats.to,
        &args,
        &[],
        vec![Cow::Owned("tw.econ.read.{{message_thread_id}}".to_string())],
    );
    let errors: CowString = format_single(
        conf_nats.errors,
        &args,
        &[],
        Cow::Owned("tw.econ.errors".to_string()),
    );
    let queue: CowString = format_single(
        conf_nats.queue,
        &args,
        &[],
        Cow::Owned("econ.reader".to_string()),
    );

    tokio::spawn(msg_reader(
        econ_reader,
        jetstream.clone(),
        write_path,
        args.clone(),
    ));
    for path in read_path {
        tokio::spawn(process_messages(
            tx.clone(),
            path,
            queue.clone(),
            nats.clone(),
        ));
    }
    for _task in config.econ.tasks.clone() {
        tokio::spawn(task(tx.clone(), _task.command, _task.delay));
    }
    tokio::spawn(check_status(
        tx.clone(),
        config.econ.check_message.clone(),
        config.econ.check_status_econ_sec,
    ));

    while let Some(message) = rx.recv().await {
        let mut attempts = 0;

        loop {
            match econ_write.send_line(&message).await {
                Ok(_) => break,
                Err(err) => {
                    error!("Error send_line to econ: {err}");

                    info!(
                        "Trying to reconnect to the server: {}/{}",
                        attempts, config.econ.reconnect.max_attempts
                    );
                    if attempts < config.econ.reconnect.max_attempts {
                        match config.econ.econ_connect(Some(&args)).await {
                            Ok(result) => {
                                econ_write = result;
                                break;
                            }
                            Err(connect_err) => {
                                error!("Error econ_connect: {connect_err}");
                                attempts += 1;
                                tokio::time::sleep(std::time::Duration::from_secs(
                                    config.econ.reconnect.sleep,
                                ))
                                .await;
                            }
                        }
                    } else {
                        info!("Max reconnect attempts reached. Giving up.");
                        let send_msg = MsgError {
                            text: message.clone(),
                            publish: errors.clone(),
                        };

                        let json = match serde_json::to_string_pretty(&send_msg) {
                            Ok(result) => result,
                            Err(err) => {
                                warn!("Error converting json to string: {err}");
                                continue;
                            }
                        };

                        jetstream
                            .publish(errors.clone().to_subject(), Bytes::from(json.to_owned()))
                            .await
                            .expect("Failed publish json")
                            .await
                            .expect("Failed publish json(2)");
                        if let Err(send_err) = tx.send(message).await {
                            error!("Failed to send message back: {send_err}");
                        }
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}
