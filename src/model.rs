use crate::errors::ConfigError;
use crate::util::get_and_format;
use anyhow::anyhow;
use async_nats::{Client, ConnectOptions, Error as NatsError};
use env_logger::Builder;
use log::{debug, info, LevelFilter};
use nestify::nest;
use serde_derive::{Deserialize, Serialize};
use serde_yaml::Value;
use std::borrow::Cow;
use std::net::{SocketAddr, ToSocketAddrs};
use std::option::Option;
use teloxide::prelude::Requester;
use teloxide::Bot as TBot;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tw_econ::Econ;

#[derive(Clone, Debug, Deserialize)]
pub enum NatsAuth {
    UserPassword { user: String, password: String },
    NKey(String),
    Token(String),
}

pub type CowString<'a> = Cow<'a, String>;

nest! {
    #[derive(Default, Clone, Deserialize)]
    pub struct Config<'a> {
        logging: Option<String>,
        pub nats:
            #[derive(Default, Clone, Deserialize)]
            pub struct NatsConfig<'c> {
                pub server: Vec<String>,
                pub auth: Option<NatsAuth>,
                #[serde(default = "default_ping_interval")]
                pub ping_interval: u64,
                #[serde(default)]
                pub tls: bool,

                // Econ & Bots
                pub from: Option<Vec<CowString<'c>>>,
                pub to: Option<Vec<CowString<'c>>>,
                pub queue: Option<CowString<'c>>,
                pub errors: Option<CowString<'c>>,

                // Handler
                pub paths: Option<Vec<
                    #[derive(Default, Clone, Deserialize)]
                    pub struct NatsHandlerPaths<'b> {
                        pub from: String,
                        pub regex: Vec<String>,
                        pub to: Vec<String>,
                        pub args: Option<Value>,
                        pub queue: Option<CowString<'b>>,
                    } ||<'c>>>,
            } ||<'a>,

        // econ
        pub econ: Option<
            #[derive(Default, Clone, Deserialize)]
            pub struct EconConfig {
                pub host: String,
                pub password: String,
                pub auth_message: Option<String>,
                #[serde(default = "default_check_status_econ_sec")]
                pub check_status_econ_sec: u64,
                #[serde(default)]
                pub check_message: String,
                #[serde(default)]
                pub tasks: Vec<
                    #[derive(Default, Clone, Deserialize)]
                    pub struct EconTasksConfig {
                        pub command: String,
                        #[serde(default = "default_tasks_delay_sec")]
                        pub delay: u64
                    }>,
                #[serde(default = "default_reconnect")]
                pub reconnect:
                    #[derive(Default, Clone, Deserialize)]
                    pub struct ReconnectConfig {
                        #[serde(default = "default_max_attempts")]
                        pub max_attempts: i64,
                        #[serde(default = "default_sleep")]
                        pub sleep: u64,
                    },
            }>,

        pub bot: Option<
            #[derive(Default, Clone, Deserialize)]
            pub struct BotConfig {
                pub token: String,
            }>,

        pub args: Option<Value>,
    }
}

fn default_ping_interval() -> u64 {
    15
}

fn default_check_status_econ_sec() -> u64 {
    5
}

fn default_tasks_delay_sec() -> u64 {
    60
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
    pub fn get_econ_addr(&self, args: Option<&Value>) -> SocketAddr {
        let args = args.unwrap_or(&Value::Null);
        get_and_format(&self.host, args, &[])
            .to_socket_addrs()
            .expect("Error create econ address")
            .next()
            .unwrap()
    }

    pub async fn econ_connect(&self, args: Option<&Value>) -> anyhow::Result<Econ> {
        let mut econ = Econ::new();

        match econ.connect(self.get_econ_addr(args)).await {
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

impl<'a> Config<'a> {
    pub async fn load_yaml(config: &str) -> Result<Self, ConfigError> {
        let mut contents = String::new();

        File::open(config)
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
            return Err(anyhow!("econ must be set, see examples"));
        }
        self.econ
            .clone()
            .unwrap()
            .econ_connect(Option::from(&self.args))
            .await
    }

    pub async fn connect_nats(&self) -> Result<Client, NatsError> {
        let connect = match &self.nats.auth {
            Some(NatsAuth::UserPassword { user, password }) => {
                ConnectOptions::new().user_and_password(user.clone(), password.clone())
            }
            Some(NatsAuth::NKey(nkey)) => ConnectOptions::new().nkey(nkey.clone()),
            Some(NatsAuth::Token(token)) => ConnectOptions::new().token(token.clone()),
            None => ConnectOptions::new(),
        };
        let nc = connect
            .ping_interval(std::time::Duration::from_secs(self.nats.ping_interval))
            .require_tls(self.nats.tls)
            .connect(&self.nats.server)
            .await?;
        debug!("Connected nats: {:?}", self.nats.server);
        Ok(nc)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsgError<'a> {
    pub text: String,
    pub publish: CowString<'a>,
}
