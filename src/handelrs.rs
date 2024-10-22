use std::process::exit;
use std::sync::Arc;
use std::time::Duration;
use async_nats::jetstream::Context;
use futures_util::StreamExt;
use log::{debug, error};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tw_econ::Econ;
use crate::model::{Env, MsgHandler};

pub async fn sender_message_to_tw(nc: async_nats::Client, js: Context, env: Env, econ: Arc<Mutex<Econ>>) {
    let mut subscriber = nc.queue_subscribe(
        format!("tw.{}", env.message_thread_id),
        format!("bridge_.{}", env.message_thread_id)
    ).await.unwrap();

    let addr = env.get_econ_addr();

    while let Some(message) = subscriber.next().await {
        let msg: &str = match std::str::from_utf8(&message.payload) {
            Ok(json_string) => json_string,
            Err(err) => {
                error!("Error converting bytes to string: {}", err);
                exit(0);
            }
        };

        if msg == "get addr" {
            let send_msg = MsgHandler {
                data: None,
                server_name: "".to_string(),
                name: None,
                message_thread_id: env.message_thread_id.clone(),
                regex_type: "addr".to_string(),
                text: addr.to_string(),
            };

            let json = match serde_json::to_string_pretty(&send_msg) {
                Ok(str) => str,
                Err(err) => {
                    error!("Json Serialize Error: {}", err);
                    exit(0);
                }
            };

            debug!("sended json to tw.messages: {}", json);
            js.publish("tw.messages", json.into())
                .await
                .expect("Error publish message to tw.messages");
            continue;
        }

        let mut econ_lock = econ.lock().await;
        match econ_lock.send_line(msg) {
            Ok(_) => {}
            Err(err) => {
                error!("Error send_line to econ: {}", err);
                exit(0);
            }
        };
    }
}


pub async fn moderator_tw(econ: Arc<Mutex<Econ>>, nc: async_nats::Client) {
    let mut subscriber = nc.subscribe("tw.moderator").await.unwrap();

    while let Some(message) = subscriber.next().await {
        let msg: &str = std::str::from_utf8(&message.payload).unwrap_or_else(|err| {
            error!("Error converting bytes to string: {}", err);
            exit(0)
        });

        debug!("send_line to econ: {}", msg);
        let mut econ_lock = econ.lock().await;
        match econ_lock.send_line(msg) {
            Ok(_) => {}
            Err(err) => {
                error!("Error send_line to econ: {}", err);
                exit(0);
            }
        };
    }
    sleep(Duration::from_millis(50)).await;
}
