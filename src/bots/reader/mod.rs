mod handlers;
mod model;

use crate::bots::model::ConfigBots;
use crate::bots::reader::handlers::message_handler;
use crate::format_values;
use crate::model::{BaseConfig, CowStr};
use log::{error, info, trace, warn};
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::*;
use teloxide::types::{MessageId, ThreadId};
use teloxide::RequestError::RetryAfter;
use tokio::sync::mpsc;
use tokio::time::sleep;

pub async fn main(config_path: String) -> anyhow::Result<()> {
    let config = ConfigBots::load_yaml(&config_path).await?;
    config.set_logging();

    let nats = config.connect_nats().await?;
    let (tx, mut rx) = mpsc::channel::<(CowStr, i64, i32)>(2048);

    let args = config.args.clone().unwrap_or_default();

    let bots = config.bot.get_bots().await;
    let mut bot_cycle = bots.iter().cycle();

    let reads_paths = format_values!(
        config.nats.from,
        &args,
        &[],
        vec![CowStr::Borrowed("tw.tg.*")]
    );
    let queue: CowStr = format_values!(
        config.nats.queue,
        &args,
        &[],
        CowStr::Borrowed("econ.reader");
        single
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
        if chat_id == -1 {
            warn!("Skipping message send attempt - invalid chat_id (-1), text: '{text}'",);
            continue;
        }

        if let Some(bot) = bot_cycle.next() {
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
    }
    Ok(())
}
