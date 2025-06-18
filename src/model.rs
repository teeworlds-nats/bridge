use crate::errors::ConfigError;
use crate::nats::{Nats, NatsAuth, NatsConfig};
use anyhow::anyhow;
use async_nats::ConnectOptions;
use env_logger::Builder;
use log::{debug, LevelFilter};
use serde::de::DeserializeOwned;
use serde_derive::{Deserialize, Serialize};
use std::borrow::Cow;
use std::option::Option;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tw_econ::Econ;

pub type CowString<'a> = Cow<'a, String>;

pub trait BaseConfig: DeserializeOwned + Sized {
    fn nats_config(&self) -> &NatsConfig<'_>;
    fn logging_config(&self) -> Option<String>;

    async fn load_yaml(config_path: &str) -> Result<Self, ConfigError> {
        let path = PathBuf::from(config_path);
        let mut contents = String::new();

        let mut file = File::open(&path)
            .await
            .map_err(|e| ConfigError::io(&path, e))?;

        file.read_to_string(&mut contents)
            .await
            .map_err(|e| ConfigError::io(&path, e))?;

        serde_yaml::from_str(&contents).map_err(|e| ConfigError::yaml(&path, e))
    }

    fn set_logging(&self) {
        let mut builder = Builder::new();
        builder.filter_level(LevelFilter::Info);

        if let Some(filters) = self.logging_config() {
            builder.parse_filters(&filters);
        }

        builder.init();
    }

    async fn connect_nats(&self) -> anyhow::Result<Nats> {
        let config = self.nats_config();
        let connect = match &config.auth {
            Some(NatsAuth::UserPassword { user, password }) => {
                ConnectOptions::new().user_and_password(user.clone(), password.clone())
            }
            Some(NatsAuth::NKey(nkey)) => ConnectOptions::new().nkey(nkey.clone()),
            Some(NatsAuth::Token(token)) => ConnectOptions::new().token(token.clone()),
            None => ConnectOptions::new(),
        }
        .connection_timeout(Duration::from_secs(30))
        .request_timeout(Some(Duration::from_secs(30)));
        let nc = connect
            .ping_interval(Duration::from_secs(config.ping_interval))
            .require_tls(config.tls)
            .connect(&config.server)
            .await?;
        debug!("Connected nats: {:?}", config.server);
        Ok(Nats::from_client(nc))
    }
    async fn econ_connect(&self) -> anyhow::Result<Econ> {
        Err(anyhow!("NOT IMPLEMENTED"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsgError<'a> {
    pub text: String,
    pub publish: CowString<'a>,
}

#[derive(Debug, Clone)]
pub struct Emoji {
    symbol: String,
    name: String,
}

#[derive(Default, Clone)]
pub struct EmojiCollection {
    emojis: Vec<Emoji>,
}

impl EmojiCollection {
    pub async fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let file = File::open(path).await?;
        let mut reader = BufReader::new(file);
        let mut emojis = Vec::new();
        let mut line = String::new();

        loop {
            let bytes_read = reader.read_line(&mut line).await?;
            if bytes_read == 0 {
                break; // EOF reached
            }

            if let Some((symbol, name)) = Self::parse_emoji_line(&line) {
                emojis.push(Emoji { symbol, name });
            }
            line.clear();
        }

        Ok(Self { emojis })
    }

    fn parse_emoji_line(line: &str) -> Option<(String, String)> {
        let mut parts = line.splitn(2, ',');
        let symbol = parts.next()?.trim().to_string();
        let name = parts.next()?.trim().to_string();

        if symbol.is_empty() || name.is_empty() {
            return None;
        }

        Some((symbol, name))
    }

    pub fn replace_symbols_with_names<'a>(&self, text: CowString<'a>) -> CowString<'a> {
        let mut result = text.to_string();

        for emoji in &self.emojis {
            result = result.replace(&emoji.symbol, &emoji.name);
        }

        Cow::Owned(result)
    }
}
