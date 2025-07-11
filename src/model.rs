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
use std::process::exit;
use std::time::Duration;
use tokio::fs;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tw_econ::Econ;

pub type CowStr<'a> = Cow<'a, str>;

const EMOJIS: &str = include_str!("emoji.txt");

pub trait BaseConfig: DeserializeOwned + Sized {
    fn nats_config(&self) -> &NatsConfig<'_>;
    fn logging_config(&self) -> Option<String>;

    async fn create_default_config(path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| ConfigError::io(path, e))?;
        }

        let mut file = File::create(path)
            .await
            .map_err(|e| ConfigError::io(path, e))?;

        file.write_all(Self::default_config().await.as_bytes())
            .await
            .map_err(|e| ConfigError::io(path, e))?;

        Ok(())
    }

    async fn load_yaml(config_path: &str) -> Result<Self, ConfigError> {
        let path = PathBuf::from(config_path);

        if !fs::try_exists(&path)
            .await
            .map_err(|e| ConfigError::io(&path, e))?
        {
            Self::create_default_config(&path).await?;

            println!("The configuration file was not found. A new file has been created along the path: {}", path.display());
            println!("Please fill it in with the necessary settings and restart the application.");
            exit(0);
        }

        let contents = fs::read_to_string(&path)
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

    async fn default_config() -> &'static str {
        ""
    }
    async fn econ_connect(&self) -> anyhow::Result<Econ> {
        Err(anyhow!("NOT IMPLEMENTED"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsgError<'a> {
    pub text: String,
    pub publish: CowStr<'a>,
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
    pub async fn new() -> anyhow::Result<Self> {
        let mut emojis = Vec::new();

        for line in EMOJIS.split("\n") {
            if let Some((symbol, name)) = Self::parse_emoji_line(line) {
                emojis.push(Emoji { symbol, name });
            }
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

    pub fn replace_symbols_with_names<'a>(&self, text: CowStr<'a>) -> CowStr<'a> {
        let mut result = text.to_string();

        for emoji in &self.emojis {
            result = result.replace(&emoji.symbol, &emoji.name);
        }

        Cow::Owned(result)
    }
}
