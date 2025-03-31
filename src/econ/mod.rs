mod handlers;
pub mod model;

use crate::econ::handlers::{check_status, msg_reader, process_messages};
use crate::model::{Config, ServerMessageData};
use crate::util::utils::err_to_string_and_exit;
use async_nats::jetstream::Context;
use async_nats::Client;
use log::info;
use tokio::sync::mpsc;

pub async fn main(config: Config, nats: Client, jetstream: Context) -> std::io::Result<()> {
    let (tx, mut rx) = mpsc::channel(64);
    let econ_reader = config
        .econ_connect()
        .await
        .expect("econ_reader failed connect");
    let mut econ_write = config
        .econ_connect()
        .await
        .expect("econ_write failed connect");
    info!("econ_reader and econ_write connected");

    let args = config.args.unwrap_or_default();
    let data = ServerMessageData::get_server_name_and_server_name(&args);

    let queue_group = format!("econ.reader.{}", &data.message_thread_id);
    let write_path: Vec<String> = data
        .replace_value(
            config
                .nats
                .to
                .unwrap_or(vec!["tw.econ.read.{{message_thread_id}}".to_string()]),
        )
        .await;
    tokio::spawn(msg_reader(econ_reader, jetstream, write_path, args));

    let read_path: Vec<String> = data
        .replace_value(config.nats.from.unwrap_or(vec![
            "tw.econ.write.{{message_thread_id}}".to_string(),
            "tw.econ.moderator".to_string(),
        ]))
        .await;
    for path in read_path {
        tokio::spawn(process_messages(
            tx.clone(),
            path,
            queue_group.clone(),
            nats.clone(),
        ));
    }
    tokio::spawn(check_status(tx.clone(), config.check_status_econ_sec));

    while let Some(message) = rx.recv().await {
        match econ_write.send_line(message).await {
            Ok(_) => {}
            Err(err) => err_to_string_and_exit("Error send_line to econ: ", Box::new(err)),
        };
    }
    Ok(())
}
