mod model;

use crate::bots::reader::model::MsgHandler;
use crate::model::Config;
use crate::util::{get, get_and_format, merge_yaml_values};
use async_nats::jetstream::Context;
use async_nats::Client;
use futures_util::StreamExt;
use log::{debug, error, info};
use teloxide::prelude::*;
use teloxide::types::ThreadId;

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
        let path_thread_id = get(&new_args, "path_thread_id", "message_thread_id");
        let thread_id = get(&new_args, &path_thread_id, "-1").parse()?;
        let message_text = get(&new_args, "message_text", "{0}: {1}");

        let text = get_and_format(&message_text, &new_args, &msg.value);

        debug!("sent message to {}({}), {}", chat_id, thread_id, text);
        bot.send_message(chat_id, text)
            .message_thread_id(ThreadId(teloxide::types::MessageId(thread_id)))
            .await?;
    }
    Ok(())
}
