use async_nats::jetstream::Context;
use async_nats::Client;
use bytes::Bytes;
use serde_derive::Serialize;
use serde_yaml::Value;

pub struct TextBuilder {
    pub text: String,
}

impl TextBuilder {
    pub fn new() -> Self {
        Self {
            text: String::default(),
        }
    }

    pub fn new_with_text(text: String) -> Self {
        Self { text }
    }

    pub fn add(&mut self, text: String) {
        if self.text.is_empty() {
            self.text.push_str(&text)
        }

        self.text.push_str(&format!(";{}", text));
    }

    pub fn to_bytes(&self) -> Bytes {
        Bytes::from(self.text.clone())
    }
}

#[derive(Clone)]
pub struct ConfigParameters {
    #[allow(dead_code)]
    pub nats: Client,
    pub jetstream: Context,
    pub send_paths: Vec<String>,
    pub args: Value,
}

#[derive(Serialize)]
pub struct CustomArgs {
    pub message_thread_id: String,
}