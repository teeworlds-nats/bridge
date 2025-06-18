mod handlers;
pub mod model;

use crate::econ::handlers::{msg_reader, process_messages, task};
use crate::econ::model::ConfigEcon;
use crate::model::{BaseConfig, CowString};
use crate::util::{format, format_single};
use log::{debug, error, info, warn};
use std::borrow::Cow;
use std::collections::HashSet;
use std::time::Duration;
use tokio::sync::mpsc;

pub async fn main(config_path: String) -> anyhow::Result<()> {
    let config = ConfigEcon::load_yaml(&config_path).await?;
    config.set_logging();

    let (tx, mut rx) = mpsc::channel(64);
    let nats = config.connect_nats().await?;

    let mut econ_write = config
        .econ_connect()
        .await
        .expect("econ_write failed connect");
    info!("econ connected");

    let args = config.args.clone().unwrap_or_default();
    let first_commands: Vec<CowString> =
        format(config.econ.first_commands.clone(), &args, &[], Vec::new());
    for command in first_commands {
        econ_write.send_line(command.to_string()).await?;
    }

    let read_path: Vec<CowString> = format(
        config.nats.from.clone(),
        &args,
        &[],
        vec![
            Cow::Owned("tw.econ.write.{{message_thread_id}}".to_string()),
            Cow::Owned("tw.econ.moderator".to_string()),
        ],
    );
    let write_path: Vec<CowString> = format(
        config.nats.to.clone(),
        &args,
        &[],
        vec![Cow::Owned("tw.econ.read.{{message_thread_id}}".to_string())],
    );
    let queue: CowString = format_single(
        config.nats.queue.clone(),
        &args,
        &[],
        Cow::Owned("econ.reader".to_string()),
    );

    let mut reader = tokio::spawn(msg_reader(
        config
            .econ_connect()
            .await
            .expect("econ_reader failed connect"),
        nats.clone(),
        write_path.clone(),
        args.clone(),
    ));
    for path in read_path {
        tokio::spawn(process_messages(
            tx.clone(),
            nats.clone(),
            path,
            queue.clone(),
        ));
    }
    for _task in config.econ.tasks.clone() {
        tokio::spawn(task(tx.clone(), _task.command, _task.delay));
    }

    let mut pending_messages = Vec::new();
    let mut reconnect_attempt = 0;
    let tasks_messages: HashSet<String> = config
        .econ
        .tasks
        .iter()
        .map(|x| x.command.clone())
        .collect();

    while let Some(message) = rx.recv().await {
        if reconnect_attempt != 0 && tasks_messages.contains(&message) {
            debug!("Skipping task command during reconnect: {message}");
            continue;
        }
        pending_messages.push(message.clone());

        while !pending_messages.is_empty() {
            match econ_write.send_line(&pending_messages[0]).await {
                Ok(_) => {
                    pending_messages.remove(0);
                    reconnect_attempt = 0;
                }
                Err(err) => {
                    error!("Error sending to econ: {err}");

                    reconnect_attempt += 1;
                    if reconnect_attempt > config.econ.reconnect.max_attempts {
                        error!(
                            "Max reconnect attempts reached. Dropping {} pending messages",
                            pending_messages.len()
                        );
                        pending_messages.clear();
                        reconnect_attempt = 0;
                        break;
                    }

                    warn!(
                        "Attempting to reconnect (attempt {}/{})",
                        reconnect_attempt, config.econ.reconnect.max_attempts
                    );

                    tokio::time::sleep(Duration::from_secs(config.econ.reconnect.sleep)).await;

                    match config.econ.econ_connect(Some(&args)).await {
                        Ok(result) => {
                            reconnect_attempt = 0;
                            reader.abort();
                            reader = tokio::spawn(msg_reader(
                                config
                                    .econ_connect()
                                    .await
                                    .expect("econ_reader failed connect"),
                                nats.clone(),
                                write_path.clone(),
                                args.clone(),
                            ));
                            econ_write = result;
                            info!("Reconnected successfully");
                            continue;
                        }
                        Err(e) => {
                            error!("Reconnect failed: {e}");
                            tokio::time::sleep(Duration::from_secs(config.econ.reconnect.sleep))
                                .await;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
