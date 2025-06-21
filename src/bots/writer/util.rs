use crate::bots::model::FormatConfig;
use crate::model::CowString;
use crate::util::escape_string;
use serde_yaml::Value;
use teloxide::prelude::Message;
use teloxide::types::{MessageCommon, MessageKind};
use crate::format_values;

pub fn get_topic_name(msg: &Message) -> String {
    if let MessageKind::Common(MessageCommon {
        reply_to_message: Some(replied_msg),
        ..
    }) = &msg.kind
    {
        if let MessageKind::ForumTopicCreated(topic_created) = &replied_msg.kind {
            return topic_created.forum_topic_created.name.clone();
        }
    }
    String::new()
}

pub fn formats<'a>(
    formats: Vec<FormatConfig>,
    args: &Value,
    text: String,
    additional_text: String,
) -> CowString<'a> {
    let mut format_text = CowString::default();
    for format in formats {
        let temp_text = format_values!(
            Some(format.format),
            args,
            &[
                text.clone(),
                format_text.to_string(),
                additional_text.clone(),
            ],
            CowString::default();
            single
        );
        format_text = if format.escape {
            escape_string(temp_text)
        } else {
            temp_text
        }
    }
    format_text
}

pub fn normalize_truncate_in_place(text: &mut String, n: usize) -> String {
    *text = text.replace(['\n', '\r'], " ");
    if let Some((idx, _)) = text.char_indices().nth(n) {
        text.truncate(idx);
    }

    text.to_string()
}
