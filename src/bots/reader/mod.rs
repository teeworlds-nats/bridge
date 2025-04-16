mod model;

use crate::bots::reader::model::MsgHandler;
use crate::model::Config;
use async_nats::jetstream::Context;
use async_nats::Client;
use futures_util::StreamExt;
use log::{debug, error, info};
use teloxide::prelude::*;
use teloxide::types::ThreadId;
use crate::util::{get, merge_yaml_values};

pub async fn main(
    config: Config,
    nats: Client,
    _jetstream: Context,
) -> Result<(), async_nats::Error> {
    let subscriber_str = config
        .nats
        .from
        .unwrap_or(vec!["tw.tg.*".to_string()])
        .first()
        .unwrap()
        .clone();
    let queue_group = "tw_tg_bot".to_string();
    let config_bot = config.bot.clone().unwrap();
    let chat_id = ChatId(config_bot.chat_id);
    let bot = config_bot.get_bot().await;
    let args = config.args.clone().unwrap_or_default();

    let mut subscriber = match nats
        .queue_subscribe(subscriber_str.clone(), queue_group)
        .await
    {
        Ok(subscriber) => subscriber,
        Err(err) => {
            panic!("Failed to subscribe to {}: {}", subscriber_str, err);
        }
    };

    info!("Subscribe to the channel: {}", subscriber_str);
    while let Some(message) = subscriber.next().await {
        debug!(
            "Message received from {}, length {}",
            message.subject, message.length
        );
        let msg: MsgHandler = match std::str::from_utf8(&message.payload) {
            Ok(json_string) => serde_json::from_str(json_string).unwrap_or_else(|err| {
                panic!("Error deserializing JSON: {}", err);
            }),
            Err(err) => {
                error!("Error converting bytes to string: {}", err);
                continue;
            }
        };

        let new_args = merge_yaml_values(&msg.args, &args);
        let value = match msg.value.len() {
            2 => msg.value.join(": "),
            _ => msg.value.join(" "),
        };
        
        let path_thread_id = get(&new_args, "path_thread_id", "message_thread_id");
        let thread_id = get(&new_args, &path_thread_id, "-1").parse()?;

        debug!("sent message to {}({}), {}", chat_id, thread_id, value);

        bot.send_message(chat_id, value)
            .message_thread_id(ThreadId(teloxide::types::MessageId(thread_id)))
            .await?;
    }
    Ok(())
}
