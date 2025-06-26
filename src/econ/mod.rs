mod enums;
mod handlers;
pub mod model;

use crate::econ::handlers::{msg_reader, process_messages};
use crate::econ::model::ConfigEcon;
use crate::format_values;
use crate::model::{BaseConfig, CowStr};
use log::{debug, error, info, warn};
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
    let first_commands: Vec<CowStr> =
        format_values!(config.econ.first_commands.clone(), &args, &[], Vec::new());
    for command in first_commands {
        econ_write.send_line(command.to_string()).await?;
    }

    let read_path: Vec<CowStr> = format_values!(
        config.nats.from.clone(),
        &args,
        &[],
        vec![
            CowStr::Borrowed("tw.econ.write.{{message_thread_id}}"),
            CowStr::Borrowed("tw.econ.moderator"),
        ]
    );
    let write_path: Vec<CowStr> = format_values!(
        config.nats.to.clone(),
        &args,
        &[],
        vec![CowStr::Borrowed("tw.econ.read.{{message_thread_id}}")]
    );
    let queue: CowStr = format_values!(
        config.nats.queue.clone(),
        &args,
        &[],
        CowStr::Borrowed("econ.reader");
        single
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
    let mut tasks = config.econ.clone().tasks;
    tasks.reverse();

    for _task in tasks {
        let task_tx = tx.clone();
        let mut task = _task.clone();
        task.init_state();

        tokio::spawn(async move {
            task.execute(&task_tx).await;
        });
    }

    let mut pending_messages = Vec::new();
    let mut reconnect_attempt = 0;

    let tasks_messages: HashSet<String> = config
        .econ
        .tasks
        .iter()
        .flat_map(|task| task.get_all_commands())
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
