use crate::model::CowString;
use anyhow::anyhow;
use async_nats::jetstream::publish::PublishAck;
use async_nats::jetstream::Context;
use async_nats::subject::ToSubject;
use async_nats::{Client, Subscriber};
use bytes::Bytes;
use log::error;
use serde_derive::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub enum NatsAuth {
    UserPassword { user: String, password: String },
    NKey(String),
    Token(String),
}

#[derive(Default, Clone, Deserialize, Debug)]
pub struct NatsConfig<'a> {
    pub server: Vec<String>,
    pub auth: Option<NatsAuth>,
    #[serde(default = "default_ping_interval")]
    pub ping_interval: u64,
    #[serde(default)]
    pub tls: bool,

    // Econ & Bots
    pub from: Option<Vec<CowString<'a>>>,
    pub to: Option<Vec<CowString<'a>>>,
    pub queue: Option<CowString<'a>>,
}

#[derive(Debug, Clone)]
pub struct Nats {
    pub nats: Client,
    pub js: Context,
}

impl Nats {
    pub fn from_client(nats: Client) -> Self {
        let js = async_nats::jetstream::new(nats.clone());
        Self { nats, js }
    }

    pub async fn subscriber<'a>(&self, patch: CowString<'a>, queue: CowString<'a>) -> Subscriber {
        match if queue.is_empty() {
            self.nats.subscribe(patch.to_subject()).await
        } else {
            self.nats
                .queue_subscribe(patch.to_subject(), queue.to_string())
                .await
        } {
            Ok(subscriber) => subscriber,
            Err(err) => {
                panic!("Failed to subscribe to \"{patch}\": {err}");
            }
        }
    }

    pub async fn publish<'a>(
        &self,
        patch: CowString<'a>,
        json: String,
    ) -> anyhow::Result<PublishAck> {
        let publish_future = self
            .js
            .publish(patch.to_subject().clone(), Bytes::from(json))
            .await
            .map_err(|e| {
                error!("NATS publish failed [subject: {patch}]: {e}");
                anyhow!("Message publish failed: {e}")
            })?;

        publish_future.await.map_err(|e| {
            error!("NATS ACK timeout [subject: {patch}]: {e}");
            anyhow!("Message delivery confirmation failed: {e}")
        })
    }
}

fn default_ping_interval() -> u64 {
    15
}
