use crate::errors::ConfigError;
use anyhow::anyhow;
use async_nats::{Client, ConnectOptions, Error as NatsError};
use env_logger::Builder;
use log::{debug, info, LevelFilter};
use nestify::nest;
use serde_derive::Deserialize;
use serde_yaml::Value;
use std::net::{SocketAddr, ToSocketAddrs};
use std::option::Option;
use teloxide::prelude::Requester;
use teloxide::Bot as TBot;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tw_econ::Econ;
// type CowStr<'a> = Cow<'a, str>;

nest! {
    #[derive(Clone, Deserialize)]
    pub struct Config {
        logging: Option<String>,
        pub nats:
            #[derive(Clone, Deserialize)]
            pub struct NatsConfig {
                pub server: String,
                pub user: Option<String>,
                pub password: Option<String>,

                // Econ & handler-auto
                pub from: Option<Vec<String>>,
                pub to: Option<Vec<String>>,

                // Handler
                pub paths: Option<Vec<
                    #[derive(Clone, Deserialize)]
                    pub struct NatsHandlerPaths {
                        pub from: Option<String>,
                        pub regex: Option<Vec<String>>,
                        pub to: Option<Vec<String>>,
                        pub args: Option<Value>,
                    }>>,
            },

        // econ
        pub econ: Option<
            #[derive(Clone, Deserialize)]
            pub struct EconConfig {
                pub host: String,
                pub password: String,
                pub auth_message: Option<String>,
                #[serde(default = "default_check_status_econ_sec")]
                pub check_status_econ_sec: u64,
                #[serde(default = "default_check_message")]
                pub check_message: String,
                #[serde(default = "default_reconnect")]
                pub reconnect:
                    #[derive(Clone, Deserialize)]
                    pub struct ReconnectConfig {
                        #[serde(default = "default_max_attempts")]
                        pub max_attempts: i64,
                        #[serde(default = "default_sleep")]
                        pub sleep: u64,
                    },
            }>,

        pub args: Option<Value>,

        pub bot: Option<
            #[derive(Clone, Deserialize)]
            pub struct BotConfig {
                pub token: String,
                pub chat_id: i64,
            }>,

    }
}

fn default_check_status_econ_sec() -> u64 {
    5
}

fn default_check_message() -> String {
    "".to_string()
}

fn default_max_attempts() -> i64 {
    10
}

fn default_sleep() -> u64 {
    5
}

fn default_reconnect() -> ReconnectConfig {
    ReconnectConfig {
        max_attempts: default_max_attempts(),
        sleep: default_sleep(),
    }
}

impl EconConfig {
    pub fn get_econ_addr(&self) -> SocketAddr {
        self.host
            .to_socket_addrs()
            .expect("Error create econ address")
            .next()
            .unwrap()
    }

    pub async fn econ_connect(&self) -> anyhow::Result<Econ> {
        let mut econ = Econ::new();

        match econ.connect(self.get_econ_addr()).await {
            Ok(_) => {}
            Err(err) => return Err(anyhow!("econ.connect, err: {}", err)),
        };
        if let Some(auth_message) = &self.auth_message {
            econ.set_auth_message(auth_message)
        }

        let authed = match econ.try_auth(&self.password).await {
            Ok(result) => result,
            Err(err) => return Err(anyhow!("econ.try_auth, err: {}", err)),
        };
        if !authed {
            return Err(anyhow!("Econ client is not authorized"));
        }

        Ok(econ)
    }
}

impl BotConfig {
    pub async fn get_bot(self) -> TBot {
        let bot = TBot::new(self.token);
        let me = bot.get_me().await.expect("Failed to execute bot.get_me");

        info!("bot {} has started", me.username());

        bot
    }
}

impl Config {
    pub async fn get_yaml() -> Result<Self, ConfigError> {
        let mut contents = String::new();

        File::open("config.yaml")
            .await?
            .read_to_string(&mut contents)
            .await?;

        let config: Self = serde_yaml::from_str(&contents).map_err(ConfigError::from)?;
        Ok(config)
    }

    pub fn set_logging(&self) {
        let mut builder = Builder::new();
        builder.filter_level(LevelFilter::Info);
        if self.logging.is_some() {
            builder.parse_filters(&self.logging.clone().unwrap());
        }
        builder.init();
    }

    pub async fn econ_connect(&self) -> anyhow::Result<Econ> {
        if self.econ.is_none() {
            return Err(anyhow!("econ must be set, see config_example.yaml"));
        }
        self.econ.clone().unwrap().econ_connect().await
    }

    pub async fn connect_nats(&self) -> Result<Client, NatsError> {
        let connect = match (self.nats.user.clone(), self.nats.password.clone()) {
            (Some(user), Some(password)) => ConnectOptions::new().user_and_password(user, password),
            _ => ConnectOptions::new(),
        };
        let nc = connect
            .ping_interval(std::time::Duration::from_secs(15))
            .connect(&self.nats.server)
            .await?;
        debug!("Connected nats: {}", self.nats.server);
        Ok(nc)
    }
}
