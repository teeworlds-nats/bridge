mod handlers;
mod model;

use crate::bots::reader::handlers::message_handler;
use crate::model::{Config, CowString};
use crate::util::{format, format_single};
use async_nats::jetstream::Context;
use async_nats::Client;
use log::{error, info, trace, warn};
use std::borrow::Cow;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::types::{MessageId, ThreadId};
use teloxide::RequestError::RetryAfter;
use tokio::sync::mpsc;
use tokio::time::sleep;

pub async fn main<'a>(config: Config<'a>, nats: Client, _jetstream: Context) -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::channel::<(CowString, i64, i32)>(64);

    let args = config.args.clone().unwrap_or_default();
    let config_bot = config.bot.clone().unwrap();
    let bot = config_bot.get_bot().await;

    let reads_paths = format(
        config.nats.from,
        &args,
        &[],
        vec![Cow::Owned("tw.tg.*".to_string())],
    );
    let queue: CowString = format_single(
        config.nats.queue,
        &args,
        &[],
        Cow::Owned("econ.reader".to_string()),
    );

    for path in reads_paths {
        tokio::spawn(message_handler(
            tx.clone(),
            path,
            queue.clone(),
            args.clone(),
            nats.clone(),
        ));
    }

    while let Some((text, chat_id, thread_id)) = rx.recv().await {
        // Валидация параметров с более информативным логом
        if chat_id == -1 {
            warn!("Skipping message send attempt - invalid chat_id (-1), text: '{text}'",);
            continue;
        }

        let msg = {
            let builder = bot.send_message(ChatId(chat_id), text.to_string());
            if thread_id != -1 {
                builder.message_thread_id(ThreadId(MessageId(thread_id)))
            } else {
                builder
            }
        };

        match msg.await {
            Ok(_) => {
                trace!("Message successfully sent to chat {chat_id}");
            }
            Err(err) => match err {
                RetryAfter(seconds) => {
                    info!("sleeping for {seconds} seconds");
                    sleep(seconds.duration()).await;
                }
                _ => {
                    error!(
                        "Failed to send message to chat {} (thread: {}): {:?}",
                        chat_id,
                        if thread_id != -1 {
                            thread_id.to_string()
                        } else {
                            "none".into()
                        },
                        err
                    );
                }
            },
        }
    }
    Ok(())
}
