use crate::bots::reader::model::MsgHandler;
use crate::model::CowString;
use crate::nats::Nats;
use crate::util::{convert, get, get_and_format, merge_yaml_values};
use futures_util::StreamExt;
use log::{debug, error, info, trace, warn};
use regex::Regex;
use serde_yaml::Value;
use std::borrow::Cow;
use tokio::sync::mpsc::Sender;

pub async fn message_handler<'a>(
    tx: Sender<(CowString<'static>, i64, i32)>,
    subscriber_str: CowString<'a>,
    queue: CowString<'a>,
    args: Value,
    nats: Nats,
) -> Result<(), async_nats::Error> {
    info!("Subscribe to the channel: \"{subscriber_str}\"");
    let mut subscriber = nats.subscriber(subscriber_str, queue).await;

    while let Some(message) = subscriber.next().await {
        debug!(
            "Message received from {}, length {}",
            message.subject, message.length
        );
        let msg = match convert::<MsgHandler>(&message.payload) {
            Some(msg) => msg,
            None => continue,
        };

        let new_args = merge_yaml_values(&msg.args, &args);
        let message_text = get(&new_args, "message_text", "{{0}}: {{1}}".to_string());
        let message_regex = get(&new_args, "message_regex", "".to_string());

        let text = if message_regex.is_empty() {
            get_and_format(&message_text, &new_args, &msg.value)
        } else {
            let regex = match Regex::new(&message_regex) {
                Ok(r) => r,
                Err(e) => {
                    error!("Failed to compile regex: \"{message_regex}\", err: {e}");
                    continue;
                }
            };

            let default_text = get_and_format(&message_text, &new_args, &msg.value);
            trace!("Applying regex \"{message_regex}\" to \"{default_text}\"");

            match regex.captures(&default_text) {
                Some(caps) => {
                    let full_match = caps.get(0).map(|m| m.as_str()).unwrap_or("");

                    let other_groups: String = caps
                        .iter()
                        .skip(1)
                        .flatten()
                        .map(|m| m.as_str())
                        .collect::<Vec<&str>>()
                        .join(" ");

                    if full_match.is_empty() {
                        warn!(
                            "Empty full match for regex '{message_regex}' in {default_text} (captured groups: {other_groups})",
                        );
                        default_text
                    } else {
                        Cow::Owned(other_groups.to_owned())
                    }
                }
                None => {
                    warn!("No matches found for regex '{message_regex}' in {default_text}",);
                    default_text
                }
            }
        };

        let not_starts_with = get(&new_args, "not_starts_with", "".to_string());
        if !not_starts_with.is_empty() && text.starts_with(&not_starts_with) {
            continue;
        }
        let chat_id = get::<i64, Value>(&new_args, "chat_id", -1);
        let thread_id = get::<i32, Value>(&new_args, "message_thread_id", -1);
        trace!("sent message to {chat_id}({thread_id}), {text}");
        tx.send((text, chat_id, thread_id)).await?;
    }

    Ok(())
}
