mod model;

use crate::model::Config;
use crate::bots::writer::model::TextBuilder;
use async_nats::jetstream::Context;
use async_nats::Client;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::RequestError;

// TODO:
async fn handle_message(
    bot: Bot,
    msg: Message,
    jetstream: Arc<Context>,
    send_paths: Vec<String>,
) -> Result<(), RequestError> {
    // TODO: msg.is_topic_message
    let thread_id = match msg.thread_id {
        None => "0".to_string(),
        Some(id) => id.to_string(),
    };
    let text = msg.text();
    if text.is_none() {
        return Ok(());
    }

    let data = TextBuilder::new();

    for path in send_paths {
        let ph = path.replace("message_thread_id", &thread_id);
        jetstream
            .publish(ph, data.to_bytes())
            .await
            .expect("Failed send to path");
    }
    Ok(())
}

pub async fn main(
    config: Config,
    nats: Client,
    jetstream: Context,
    bot: Bot,
) -> Result<(), async_nats::Error> {
    let send_paths = Arc::new(
        config
            .nats
            .to
            .unwrap_or(vec!["tw.econ.write.{{message_thread_id}}".to_string()]),
    );

    let jetstream = Arc::new(jetstream);

    teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        Ok(())
    })
    .await;
    Ok(())
}
