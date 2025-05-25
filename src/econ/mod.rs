mod handlers;
pub mod model;

use crate::econ::handlers::{check_status, msg_reader, process_messages, task};
use crate::model::{Config, CowString, MsgError};
use crate::util::get_and_format;
use async_nats::jetstream::Context;
use async_nats::subject::ToSubject;
use async_nats::Client;
use bytes::Bytes;
use log::{error, info, warn};
use std::borrow::Cow;
use tokio::sync::mpsc;

fn default_from<'a>() -> Vec<CowString<'a>> {
    vec![
        Cow::Owned("tw.econ.write.{{message_thread_id}}".to_string()),
        Cow::Owned("tw.econ.moderator".to_string()),
    ]
}

pub async fn main<'a>(config: Config<'a>, nats: Client, jetstream: Context) -> std::io::Result<()> {
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
    let conf_econ = config.econ.unwrap();
    let args = config.args.clone().unwrap_or_default();

    let write_path: Vec<String> = conf_nats
        .to
        .unwrap_or(vec![Cow::Owned(
            "tw.econ.read.{{message_thread_id}}".to_string(),
        )])
        .iter()
        .map(|x| get_and_format(x, &args, &Vec::new()).to_string())
        .collect();
    let errors: CowString = conf_nats
        .errors
        .unwrap_or(Cow::Owned("tw.econ.errors".to_string()));
    let read_path: Vec<CowString> = conf_nats
        .from
        .unwrap_or(default_from())
        .iter()
        .map(|x| get_and_format(x, &args, &Vec::new()))
        .collect();

    tokio::spawn(msg_reader(econ_reader, jetstream.clone(), write_path, args));
    for path in read_path {
        tokio::spawn(process_messages(
            tx.clone(),
            path,
            "econ.reader".to_string(),
            nats.clone(),
        ));
    }
    for _task in conf_econ.tasks.clone() {
        tokio::spawn(task(tx.clone(), _task.command, _task.delay));
    }
    tokio::spawn(check_status(
        tx.clone(),
        conf_econ.check_message.clone(),
        conf_econ.check_status_econ_sec,
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
                        attempts, conf_econ.reconnect.max_attempts
                    );
                    if attempts < conf_econ.reconnect.max_attempts {
                        match conf_econ.econ_connect().await {
                            Ok(result) => {
                                econ_write = result;
                                break;
                            }
                            Err(connect_err) => {
                                error!("Error econ_connect: {connect_err}");
                                attempts += 1;
                                tokio::time::sleep(std::time::Duration::from_secs(
                                    conf_econ.reconnect.sleep,
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
