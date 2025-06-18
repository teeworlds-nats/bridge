mod model;
mod util;

use crate::bots::model::{ConfigBots, Formats};
use crate::bots::writer::model::ConfigParameters;
use crate::bots::writer::util::{formats, get_topic_name, normalize_truncate_in_place};
use crate::handler::model::MsgHandler;
use crate::model::{BaseConfig, CowString, EmojiCollection};
use crate::util::{format, merge_yaml_values};
use log::{debug, warn};
use serde_yaml::{to_value, Value};
use std::borrow::Cow;
use teloxide::prelude::*;
use teloxide::RequestError;

fn msg_format<'a>(
    msg: &Message,
    args: &'a Value,
    fr: Formats,
    reply: bool,
) -> Option<CowString<'a>> {
    let format = if reply { fr.reply } else { fr.text };

    if let Some(sticker) = msg.sticker() {
        return Some(formats(
            format,
            args,
            sticker.clone().emoji.unwrap_or_default(),
            fr.sticker,
        ));
    }

    if let Some(media) = msg.media_group_id() {
        return Some(formats(format, args, media.to_string(), fr.media));
    }

    if let Some(text) = msg.text() {
        return Some(formats(
            format,
            args,
            normalize_truncate_in_place(&mut text.to_string(), 500),
            String::new(),
        ));
    }

    None
}

async fn handle_message(msg: Message, cfg: ConfigParameters) -> Result<(), RequestError> {
    let args = {
        let mut args = merge_yaml_values(&to_value(msg.clone()).unwrap_or_default(), &cfg.args);
        args["message_thread_id"] = Value::String(
            msg.thread_id
                .map_or_else(|| "-1".to_string(), |id| id.to_string()),
        );
        args["server_name"] = Value::String(get_topic_name(&msg));
        args["chat_id"] = Value::from(msg.chat.id.0);
        args["econ_divide"] = Value::from(true);
        args
    };

    let write_paths = format(
        cfg.send_paths,
        &args,
        &[],
        vec![Cow::Owned("tw.tg.*".to_string())],
    );

    let mut texts = Vec::new();
    if let Some(reply) = msg.reply_to_message() {
        let result = msg_format(reply, &args, cfg.formats.clone(), true);
        if let Some(result) = result {
            texts.push(cfg.emojis.replace_symbols_with_names(result));
        }
    }
    if let Some(result) = msg_format(&msg, &args, cfg.formats, false) {
        texts.push(cfg.emojis.replace_symbols_with_names(result));
    }

    let data = MsgHandler::get_json(
        texts.iter().map(|x| x.to_string()).collect(),
        String::new(),
        &args,
    )
    .await;
    debug!("send {data} to {write_paths:?}:");
    for path in write_paths {
        cfg.nats.publish(path, data.to_owned()).await.ok();
    }
    Ok(())
}

async fn handler_bot(bot: Bot, parameters: ConfigParameters) -> anyhow::Result<()> {
    let handler = Update::filter_message().branch(
        dptree::filter(|msg: Message| msg.text().is_some() || msg.is_topic_message).endpoint(
            |cfg: ConfigParameters, msg: Message| async move {
                handle_message(msg, cfg).await?;
                respond(())
            },
        ),
    );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![parameters])
        .default_handler(|upd| async move {
            warn!("Unhandled update: {upd:?}");
        })
        .error_handler(LoggingErrorHandler::with_custom_text(
            "An error has occurred in the dispatcher",
        ))
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
    Ok(())
}

pub async fn main(config_path: String) -> anyhow::Result<()> {
    let config = ConfigBots::load_yaml(&config_path).await?;
    config.set_logging();

    let nats = config.connect_nats().await?;
    let send_paths = config.nats.to.unwrap_or(vec![CowString::Owned(
        "tw.econ.write.{{message_thread_id}}".to_string(),
    )]);
    let emojis = EmojiCollection::from_file("emoji.txt").await?;

    let parameters = ConfigParameters {
        emojis,
        nats,
        send_paths,
        formats: config.format,
        args: config.args.unwrap_or_default(),
    };

    let bots = config.bot.get_bots().await;
    for bot in bots {
        tokio::spawn(handler_bot(bot, parameters.clone()));
    }

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}
