use crate::errors::ConfigError;
use anyhow::anyhow;
use async_nats::{Client, ConnectOptions, Error as NatsError};
use env_logger::Builder;
use log::{debug, LevelFilter};
use serde::de::DeserializeOwned;
use serde_derive::{Deserialize, Serialize};
use std::borrow::Cow;
use std::option::Option;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tw_econ::Econ;

pub trait BaseConfig: DeserializeOwned + Sized {
    fn nats_config(&self) -> &NatsConfig<'_>;
    fn logging_config(&self) -> Option<String>;

    async fn load_yaml(config_path: &str) -> Result<Self, ConfigError> {
        let mut contents = String::new();
        File::open(config_path)
            .await?
            .read_to_string(&mut contents)
            .await?;

        serde_yaml::from_str(&contents).map_err(|e| e.into())
    }

    fn set_logging(&self) {
        let mut builder = Builder::new();
        builder.filter_level(LevelFilter::Info);

        if let Some(filters) = self.logging_config() {
            builder.parse_filters(&filters);
        }

        builder.init();
    }

    async fn connect_nats(&self) -> Result<Client, NatsError> {
        let config = self.nats_config();
        let connect = match &config.auth {
            Some(NatsAuth::UserPassword { user, password }) => {
                ConnectOptions::new().user_and_password(user.clone(), password.clone())
            }
            Some(NatsAuth::NKey(nkey)) => ConnectOptions::new().nkey(nkey.clone()),
            Some(NatsAuth::Token(token)) => ConnectOptions::new().token(token.clone()),
            None => ConnectOptions::new(),
        };
        let nc = connect
            .ping_interval(std::time::Duration::from_secs(config.ping_interval))
            .require_tls(config.tls)
            .connect(&config.server)
            .await?;
        debug!("Connected nats: {:?}", config.server);
        Ok(nc)
    }
    async fn econ_connect(&self) -> anyhow::Result<Econ> {
        Err(anyhow!("NOT IMPLEMENTED"))
    }
}

#[derive(Clone, Debug, Deserialize)]
pub enum NatsAuth {
    UserPassword { user: String, password: String },
    NKey(String),
    Token(String),
}

pub type CowString<'a> = Cow<'a, String>;

#[derive(Default, Clone, Deserialize)]
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
    pub errors: Option<CowString<'a>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsgError<'a> {
    pub text: String,
    pub publish: CowString<'a>,
}

fn default_ping_interval() -> u64 {
    15
}
