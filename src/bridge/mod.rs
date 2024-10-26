pub mod handlers;

use async_nats::Client;
use async_nats::jetstream::Context;
use log::{debug, info};
use crate::bridge::handlers::{moderator_tw, sender_message_to_tw};
use crate::model::{Env, MsgBridge};
use crate::util::utils::econ_connect;

pub async fn main(env: Env, nats: Client, jetstream: Context) -> Result<(), async_nats::Error>  {
    if env.message_thread_id.is_none() || env.server_name.is_none() {
        panic!("econ_password and server_name must be set");
    }

    let message_thread_id = env.message_thread_id.clone().unwrap();
    let server_name = env.server_name.clone().unwrap();
    let publish_stream = "tw.econ.read.".to_owned() + &message_thread_id.clone();

    let econ_read = econ_connect(env.clone()).await?;
    let econ_write = econ_connect(env.clone()).await?;
    info!("econ connected");


    tokio::spawn(sender_message_to_tw(nats.clone(), message_thread_id.clone(), econ_write.clone()));
    tokio::spawn(moderator_tw(econ_write.clone(), nats.clone()));

    loop {
        let Some(message) = (match econ_read.lock().await.recv_line(true) {
            Ok(result) => { result }
            Err(err) => {
                panic!("panic from loop: {}", err);
            }
        }) else { continue};

        let send_msg = MsgBridge {
            server_name: server_name.clone(),
            message_thread_id: message_thread_id.clone(),
            text: message
        };
        let json = serde_json::to_string_pretty(&send_msg).expect("Failed Serialize Msg");

        debug!("send json to tw.econ.read.(id): {}", json);
        jetstream.publish(publish_stream.clone(), json.into())
            .await
            .expect("Error publish message to tw.econ.read.(id)");
    }
}

