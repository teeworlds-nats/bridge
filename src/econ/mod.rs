mod handlers;
pub mod model;

use crate::econ::handlers::{check_status, msg_reader, process_messages};
use crate::model::Config;
use crate::util::utils::{econ_connect, err_to_string_and_exit, replace_value};
use async_nats::jetstream::Context;
use async_nats::Client;
use log::info;
use serde_yaml::Value;
use tokio::sync::mpsc;

pub async fn main(env: Config, nats: Client, jetstream: Context) -> std::io::Result<()> {
    let (tx, mut rx) = mpsc::channel(32);
    let econ_reader = econ_connect(env.clone()).await?;
    let mut econ_write = econ_connect(env.clone()).await?;
    info!("econ_reader and econ_write connected");

    let args = env.args;
    let server_name = args
        .get("server_name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let message_thread_id = args
        .get("message_thread_id")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .to_string();

    let queue_group = format!("econ.reader.{}", &message_thread_id);
    let read_path: Vec<String> = replace_value(
        env.nats.from.unwrap_or(vec![
            "tw.econ.write.{{message_thread_id}}".to_string(),
            "tw.econ.moderator".to_string(),
        ]),
        &message_thread_id,
        &server_name,
    )
    .await;

    let write_path: Vec<String> = replace_value(
        env.nats
            .to
            .unwrap_or(vec!["tw.econ.read.{{message_thread_id}}".to_string()]),
        &message_thread_id,
        &server_name,
    )
    .await;

    tokio::spawn(msg_reader(econ_reader, jetstream, write_path, args));
    for path in read_path {
        tokio::spawn(process_messages(
            tx.clone(),
            path,
            queue_group.clone(),
            nats.clone(),
        ));
    }
    tokio::spawn(check_status(tx.clone(), env.check_status_econ));

    while let Some(message) = rx.recv().await {
        match econ_write.send_line(message).await {
            Ok(_) => {}
            Err(err) => err_to_string_and_exit("Error send_line to econ: ", Box::new(err)),
        };
    }
    Ok(())
}
