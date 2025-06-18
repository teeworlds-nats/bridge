use crate::bots::model::Formats;
use crate::model::{CowString, EmojiCollection};
use crate::nats::Nats;
use serde_yaml::Value;

#[derive(Clone)]
pub struct ConfigParameters {
    pub emojis: EmojiCollection,
    pub nats: Nats,
    pub send_paths: Vec<CowString<'static>>,
    pub formats: Formats,
    pub args: Value,
}
