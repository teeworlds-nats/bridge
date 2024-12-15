pub mod handlers;

use crate::handler::handlers::chat_handler;
use crate::model::{EnvHandler, MsgBridge};
use async_nats::jetstream::Context;
use async_nats::Client;
use futures::StreamExt;
use log::{debug, error, info};
use std::process::exit;

pub async fn main(
    env: EnvHandler,
    nats: Client,
    jetstream: Context,
) -> Result<(), async_nats::Error> {
    let mut subscriber = nats
        .queue_subscribe("tw.econ.read.*", "handler".to_string())
        .await?;

    info!("Handler started");
    while let Some(message) = subscriber.next().await {
        debug!(
            "message received from {}, length {}",
            message.subject, message.length
        );
        let msg: MsgBridge = match std::str::from_utf8(&message.payload) {
            Ok(json_string) => serde_json::from_str(json_string).unwrap_or_else(|err| {
                error!("Error deserializing JSON: {}", err);
                exit(1);
            }),
            Err(err) => {
                error!("Error converting bytes to string: {}", err);
                exit(1);
            }
        };

        for pattern in env.paths.iter() {
            if pattern.read == message.subject.to_string() {
                continue;
            };

            for regex in &pattern.regex {
                if let Some(caps) = regex.captures(&msg.text) {
                    let json = chat_handler(&msg, &env, caps, pattern).await;

                    if json.is_empty() {
                        break;
                    }

                    debug!("sent json to {:?}: {}", pattern.write, json);
                    for write_path in &pattern.write {
                        let path = write_path
                            .replacen("{{message_thread_id}}", &msg.message_thread_id.clone().unwrap_or(msg.server_name.clone()), 1)
                            .replacen("{{server_name}}", &msg.server_name, 1);

                        jetstream
                            .publish(path, json.clone().into())
                            .await
                            .expect("Error publish message to tw.messages");
                    }
                    break;
                }
            }
        }
    }

    Ok(())
}
