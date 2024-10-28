pub mod handlers;

use std::process::exit;
use async_nats::Client;
use async_nats::jetstream::Context;
use log::{debug, info};
use crate::bridge::handlers::{moderator_tw, sender_message_to_tw};
use crate::model::{Env, MsgBridge};
use crate::util::utils::{econ_connect, send_message};

pub async fn main(env: Env, nats: Client, jetstream: Context) -> Result<(), async_nats::Error>  {
    if env.message_thread_id.is_none() || env.server_name.is_none() {
        eprintln!("econ_password and server_name must be set");
        exit(0);
    }

    let message_thread_id = env.message_thread_id.clone().unwrap();
    let server_name = env.server_name.clone().unwrap();
    let publish_stream = "tw.econ.read.".to_owned() + &message_thread_id.clone();

    let econ = econ_connect(env.clone()).await?;
    let econ_write = econ_connect(env.clone()).await?;
    info!("econ connected");

    tokio::spawn(sender_message_to_tw(nats.clone(), message_thread_id.clone(), econ_write.clone()));
    tokio::spawn(moderator_tw(econ_write.clone(), nats.clone()));

    loop {
        let line = match econ.lock().await.recv_line(true) {
            Ok(result) => { result }
            Err(err) => {
                eprintln!("err from loop: {}", err);
                exit(0);
            }
        };

        if line.is_none() {
            continue
        }

        if let Some(message) = line {
            debug!("Recevered line from econ: {}", message);
            let send_msg = MsgBridge {
                server_name: server_name.clone(),
                message_thread_id: message_thread_id.clone(),
                text: message,
            };

            let json = serde_json::to_string_pretty(&send_msg)?;

            debug!("Sending JSON to tw.econ.read.(id): {}", json);
            if let Err(e) = send_message(&json, &publish_stream, &jetstream).await {
                eprintln!("Failed to send message: {}", e);
            }
        }
    }
}

