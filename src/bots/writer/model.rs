use crate::bots::model::FormatsConfigs;
use crate::model::{CowStr, EmojiCollection};
use crate::nats::Nats;
use serde_yaml::Value;

#[derive(Clone)]
pub struct ConfigParameters {
    pub emojis: EmojiCollection,
    pub nats: Nats,
    pub send_paths: Vec<CowStr<'static>>,
    pub formats: FormatsConfigs,
    pub args: Value,
}
