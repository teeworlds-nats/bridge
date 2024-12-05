pub mod handlers;

use crate::econ::handlers::{check_status, msg_reader, process_messages};
use crate::model::Env;
use crate::util::utils::{econ_connect, err_to_string_and_exit, get_path};
use async_nats::jetstream::Context;
use async_nats::Client;
use log::{error, info};
use std::process::exit;
use tokio::sync::mpsc;

pub async fn main(env: Env, nats: Client, jetstream: Context) -> std::io::Result<()> {
    if env.message_thread_id.is_none() || env.server_name.is_none() {
        error!("econ_password and server_name must be set");
        exit(1);
    }

    let Some(message_thread_id) = env.message_thread_id.clone() else {
        error!("message_thread_id is none");
        exit(1)
    };

    let server_name = env.server_name.clone().unwrap_or_default();

    let (tx, mut rx) = mpsc::channel(32);
    let econ_reader = econ_connect(env.clone()).await?;
    let mut econ_write = econ_connect(env.clone()).await?;
    info!("econ_reader and econ_write connected");

    let queue_group = format!("econ.reader.{}", message_thread_id);
    let read_path: Vec<String> = get_path(
        env.nats.read_path,
        vec![
            "tw.econ.write.{{message_thread_id}}".to_string(),
            "tw.econ.moderator".to_string(),
        ],
    )
    .iter()
    .map(|x| {
        x.replacen("{{message_thread_id}}", &message_thread_id, 1)
            .replacen("{{server_name}}", &server_name, 1)
    })
    .collect();
    let write_path: Vec<String> = get_path(
        env.nats.write_path,
        vec!["tw.econ.read.{{message_thread_id}}".to_string()],
    )
    .iter()
    .map(|x| {
        x.replacen("{{message_thread_id}}", &message_thread_id, 1)
            .replacen("{{server_name}}", &server_name, 1)
    })
    .collect();

    tokio::spawn(msg_reader(
        econ_reader,
        jetstream,
        write_path,
        message_thread_id.clone(),
        server_name,
    ));
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
