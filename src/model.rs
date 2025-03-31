use crate::util::errors::ConfigError;
use async_nats::{Client, ConnectOptions, Error as NatsError};
use env_logger::Builder;
use log::{debug, error, LevelFilter};
use nestify::nest;
use regex::Regex;
use serde_derive::Deserialize;
use serde_yaml::Value;
use std::net::{SocketAddr, ToSocketAddrs};
use std::option::Option;
use std::process::exit;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tw_econ::Econ;
// type CowStr<'a> = Cow<'a, str>;

pub struct RegexData {
    pub name: String,
    pub regex: Regex,
}

impl RegexData {
    pub fn new(name: String, regex: String) -> Self {
        Self {
            name,
            regex: Regex::new(&regex).unwrap(),
        }
    }
}

pub struct ServerMessageData {
    #[allow(dead_code)]
    pub path_server_name: String,
    #[allow(dead_code)]
    pub path_thread_id: String,
    pub server_name: String,
    pub message_thread_id: String,
}

impl ServerMessageData {
    pub fn get_server_name_and_server_name(args: &Value) -> Self {
        fn get(args: &Value, index: &str, default: &str) -> String {
            args.get(index)
                .and_then(Value::as_str)
                .unwrap_or(default)
                .to_string()
        }
        let path_server_name = get(args, "path_server_name", "server_name");
        let path_thread_id = get(args, "path_thread_id", "message_thread_id");

        let server_name = get(args, &path_server_name, "");
        let message_thread_id = get(args, &path_thread_id, "-1");

        Self {
            path_server_name,
            path_thread_id,
            server_name,
            message_thread_id,
        }
    }

    pub async fn replace_value<T>(&self, input: T) -> Vec<String>
    where
        T: IntoIterator<Item = String>,
    {
        input
            .into_iter()
            .map(|item| {
                item.replace(
                    &format!("{{{{{}}}}}", &self.path_thread_id),
                    &self.message_thread_id,
                )
                .replace(
                    &format!("{{{{{}}}}}", &self.path_server_name),
                    &self.server_name,
                )
            })
            .collect()
    }

    pub async fn replace_value_single(&self, value: &str) -> String {
        value
            .replace(
                &format!("{{{{{}}}}}", &self.path_thread_id),
                &self.message_thread_id,
            )
            .replace(
                &format!("{{{{{}}}}}", &self.path_server_name),
                &self.server_name,
            )
    }
}

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

        pub check_status_econ_sec: Option<u64>,
        pub econ: Option<
            #[derive(Clone, Deserialize)]
            pub struct EconConfig {
                pub host: String,
                pub password: Option<String>,
                pub auth_message: Option<String>,
            }>,

        pub args: Option<Value>,
    }
}

impl Config {
    pub async fn get_yaml() -> Result<Self, ConfigError> {
        let mut contents = String::new();

        File::open("config.yaml")
            .await?
            .read_to_string(&mut contents)
            .await?;

        let env: Config = serde_yaml::from_str(&contents).map_err(ConfigError::from)?;
        Ok(env)
    }

    pub fn set_logging(&self) {
        let mut builder = Builder::new();
        builder.filter_level(LevelFilter::Info);
        if self.logging.is_some() {
            builder.parse_filters(&self.logging.clone().unwrap());
        }
        builder.init();
    }

    pub fn get_econ_addr(&self) -> SocketAddr {
        let Some(econ) = self.econ.clone() else {
            exit(-1)
        };
        econ.host
            .to_socket_addrs()
            .expect("Error create econ address")
            .next()
            .unwrap()
    }

    pub async fn econ_connect(&self) -> std::io::Result<Econ> {
        let mut econ = Econ::new();
        if self.econ.is_none() {
            error!("econ must be set, see config_example.yaml");
            exit(1);
        }
        let econ_env = self.econ.clone().unwrap();

        if econ_env.password.is_none() {
            error!("econ.password must be set");
            exit(1);
        }

        econ.connect(self.get_econ_addr()).await?;
        if let Some(auth_message) = econ_env.auth_message {
            econ.set_auth_message(auth_message)
        }

        let authed = econ.try_auth(econ_env.password.unwrap()).await?;
        if !authed {
            error!("Econ client is not authorized");
            exit(1);
        }

        Ok(econ)
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
