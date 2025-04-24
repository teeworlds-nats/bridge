mod model;

use crate::bots::writer::model::{ConfigParameters, CustomArgs, TextBuilder};
use crate::model::Config;
use crate::util::{get_and_format, merge_yaml_values};
use async_nats::jetstream::Context;
use async_nats::Client;
use bytes::Bytes;
use log::debug;
use serde_yaml::to_value;
use teloxide::prelude::*;
use teloxide::RequestError;

// Todo:
async fn handle_message(msg: Message, cfg: ConfigParameters) -> Result<(), RequestError> {
    let thread_id = msg
        .thread_id
        .map_or_else(|| "-1".to_string(), |id| id.to_string());
    let text = format!("say {}", msg.text().unwrap_or_default());
    let args = to_value(CustomArgs {
        message_thread_id: thread_id,
    })
    .unwrap_or_default();

    let new_args = merge_yaml_values(&args, &cfg.args);
    let write_paths: Vec<String> = cfg
        .send_paths
        .iter()
        .map(|x| get_and_format(x, &new_args, &[]).to_string())
        .collect();
    debug!("send \"{}\" to {:?}:", text, write_paths);

    let text = TextBuilder { text };
    let data = Bytes::from(text.to_bytes());
    for path in cfg.send_paths {
        cfg.jetstream
            .publish(path, data.clone())
            .await
            .expect("bot writer send error");
    }
    Ok(())
}

pub async fn main(
    config: Config,
    nats: Client,
    jetstream: Context,
) -> Result<(), async_nats::Error> {
    let send_paths = config
        .nats
        .to
        .unwrap_or(vec!["tw.econ.write.{{message_thread_id}}".to_string()]);
    let config_bot = config.bot.clone().unwrap();
    let bot = config_bot.get_bot().await;
    let parameters = ConfigParameters {
        nats,
        jetstream,
        send_paths,
        args: config.args.unwrap_or_default(),
    };

    let handler = Update::filter_message().branch(
        dptree::filter(|msg: Message| msg.text().is_some() || msg.is_topic_message).endpoint(
            |cfg: ConfigParameters, msg: Message| async move {
                handle_message(msg, cfg).await?;
                respond(())
            },
        ),
    );

    Dispatcher::builder(bot, handler)
        // Here you specify initial dependencies that all handlers will receive; they can be
        // database connections, configurations, and other auxiliary arguments. It is similar to
        // `actix_web::Extensions`.
        .dependencies(dptree::deps![parameters])
        .default_handler(|_upd| async move { /* log::warn!("Unhandled update: {:?}", upd); */ })
        // If the dispatcher fails for some reason, execute this handler.
        .error_handler(LoggingErrorHandler::with_custom_text(
            "An error has occurred in the dispatcher",
        ))
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
