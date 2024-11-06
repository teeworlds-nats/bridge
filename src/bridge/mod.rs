pub mod handlers;

use std::process::exit;
use async_nats::Client;
use async_nats::jetstream::Context;
use log::{error, info};
use tokio::sync::mpsc;
use crate::bridge::handlers::{check_status, msg_reader, process_messages};
use crate::model::{Env};
use crate::util::utils::{econ_connect, err_to_string_and_exit};


pub async fn main(env: Env, nats: Client, jetstream: Context) -> std::io::Result<()>  {
    if env.message_thread_id.is_none() || env.server_name.is_none() {
        error!("econ_password and server_name must be set");
        exit(1);
    }

    let Some(message_thread_id) = env.message_thread_id.clone() else { error!("message_thread_id is none"); exit(1) };

    let (tx, mut rx) = mpsc::channel(32);
    let econ_reader = econ_connect(env.clone()).await?;
    let mut econ_write = econ_connect(env.clone()).await?;
    info!("econ_reader and econ_write connected");

    let queue_group = format!("econ.reader.{}", message_thread_id);
    let econ_subscriber = nats.queue_subscribe(format!("tw.econ.write.{}", message_thread_id), queue_group.clone()).await.unwrap();
    let moderators_events = nats.queue_subscribe("tw.econ.moderator", queue_group).await.unwrap();

    tokio::spawn(msg_reader(econ_reader, jetstream, message_thread_id.clone(), env.server_name.clone().unwrap()));
    tokio::spawn(process_messages(tx.clone(), econ_subscriber));
    tokio::spawn(process_messages(tx.clone(), moderators_events));
    tokio::spawn(check_status(tx.clone(), env.check_status_econ));

    while let Some(message) = rx.recv().await {
        match econ_write.send_line(message).await {
            Ok(_) => {}
            Err(err) => {
                err_to_string_and_exit("Error send_line to econ: ", Box::new(err))
            }
        };
    }
    Ok(())
}

