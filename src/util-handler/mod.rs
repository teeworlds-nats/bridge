mod handlers;

use std::process::exit;
use async_nats::Client;
use async_nats::jetstream::Context;
use futures::StreamExt;
use log::{info, debug, warn};
use once_cell::sync::Lazy;
use crate::model::{Env, HandlerPaths, MsgBridge, MsgUtil};
use crate::util::utils::get_path;
use crate::util_handler::handlers::process_rcon;


pub static DD_PATTERNS_UTIL: Lazy<Vec<HandlerPaths>> = Lazy::new(|| {
    vec![
        HandlerPaths::new(r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} I server: ClientI[dD]=\d+ rcon='([^']+)'$"),
        HandlerPaths::new(r"^\[server]: ClientID=\d+ rcon='([^']+)'$"),
        HandlerPaths::new(r"\[.*?]\[server]: ClientID=.* rcon='(.*?)'"),
    ]
});


pub async fn main(env: Env, nats: Client, jetstream: Context) -> Result<(), async_nats::Error> {
    let read_path= get_path(
        env.nats.read_path.clone(),
        vec!("tw.econ.read.*".to_string())
    );

    if read_path.len() > 1 {
        warn!("handler-util can only work with one patch all others will be ignored: {:?}", read_path)
    }

    let mut subscriber = nats.queue_subscribe(read_path.get(0).unwrap().to_string(), "util".to_string()).await?;
    let (commands_sync, commands_log) = env.get_commands();

    info!("Handler started");
    let mut rcon_last = "".to_string();
    let subject = get_path(
        env.nats.write_path,
        vec!("tw.events".to_string())
    );


    while let Some(message) = subscriber.next().await {
        let msg: MsgBridge = match std::str::from_utf8(&message.payload) {
            Ok(json_string) => serde_json::from_str(json_string).unwrap_or_else(|err| {
                eprintln!("Error deserializing JSON: {}", err);
                exit(1);
            }),
            Err(err) => {
                eprintln!("Error converting bytes to string: {}", err);
                exit(1);
            }
        };

        let text_clone = msg.text.clone();
        for pattern in DD_PATTERNS_UTIL.iter() {
            let regex = pattern.regex.get(0).unwrap();

            if !regex.is_match(&text_clone) {
                continue;
            }

            let caps = regex.captures(&text_clone).unwrap();
            let value = process_rcon(caps, &commands_sync, &commands_log).await;

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
                        eprintln!("Json Serialize Error: {}", err);
                        break
                    }
                };

                debug!("Sending JSON to {:?}: {}", subject, json);
                for send_path in subject.clone() {
                    jetstream.publish(send_path, json.clone().into()).await?.await?;
                }
            }

            if value.sync {
                debug!("Sending DATA to tw.econ.moderator: {}", text);
                jetstream.publish("tw.econ.moderator", text.clone().into()).await?.await?;
            }

            rcon_last = text;
            break
        }
    }


    Ok(())
}