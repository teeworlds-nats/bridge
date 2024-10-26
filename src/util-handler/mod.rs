mod handlers;

use std::process::exit;
use async_nats::Client;
use async_nats::jetstream::Context;
use futures::StreamExt;
use log::{info, error, debug};
use crate::model::{Env, MsgBridge, MsgUtil};
use crate::util::patterns::DD_PATTERNS_UTIL;
use crate::util_handler::handlers::process_rcon;

pub async fn main(env: Env, nats: Client, jetstream: Context) -> Result<(), async_nats::Error> {
    let mut subscriber = nats.queue_subscribe("tw.econ.read.*", "util".to_string()).await?;

    let (commands_sync, commands_log) = env.get_commands();

    info!("Handler started");
    let mut rcon_last = "".to_string();
    while let Some(message) = subscriber.next().await {
        let msg: MsgBridge = match std::str::from_utf8(&message.payload) {
            Ok(json_string) => serde_json::from_str(json_string).unwrap_or_else(|err| {
                error!("Error deserializing JSON: {}", err);
                exit(0);
            }),
            Err(err) => {
                error!("Error converting bytes to string: {}", err);
                exit(0);
            }
        };

        let text_clone = msg.text.clone();
        for pattern in DD_PATTERNS_UTIL.iter() {
            if !pattern.regex.is_match(&text_clone) {
                continue;
            }

            let caps = pattern.regex.captures(&text_clone).unwrap();
            let value = match pattern.name.as_ref() {
                "rcon" => process_rcon(caps, &commands_sync, &commands_log).await,
                _ => {continue}
            };

            let Some(text) = value.text else { continue };

            if rcon_last == text {
                continue;
            }

            if value.log {
                let send_msg = MsgUtil {
                    server_name: msg.server_name.clone(),
                    rcon: text.clone().to_string(),
                };

                let json = match serde_json::to_string_pretty(&send_msg) {
                    Ok(str) => { str }
                    Err(err) => {
                        error!("Json Serialize Error: {}", err);
                        break
                    }
                };

                debug!("send to tw.events: {}", json);
                jetstream.publish("tw.events", json.into())
                    .await
                    .expect("Error publish message to tw.events");
            }

            if value.sync {
                debug!("send to tw.econ.moderator: {}", &text);
                jetstream.publish("tw.econ.moderator", text.clone().into())
                    .await
                    .expect("Error publish message to tw.moderator");
            }

            rcon_last = text;

            break
        }
    }


    Ok(())
}