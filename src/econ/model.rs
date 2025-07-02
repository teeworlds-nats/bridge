use crate::econ::enums::Task;
use crate::format::formatting;
use crate::model::{BaseConfig, CowStr};
use crate::nats::NatsConfig;
use anyhow::anyhow;
use log::warn;
use nestify::nest;
use serde_derive::{Deserialize, Serialize};
use serde_yaml::Value;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use tokio::sync::Mutex;
use tw_econ::Econ;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsgBridge {
    pub text: String,
    pub args: Value,
}

impl MsgBridge {
    pub fn json(self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(&self)
    }
}

nest! {
    #[derive(Default, Clone, Deserialize)]
    pub struct ConfigEcon<'a> {
        logging: Option<String>,
        pub nats: NatsConfig<'a>,

        pub econ:
            #[derive(Default, Clone, Deserialize)]
            pub struct EconConfig {
                pub host: String,
                pub password: String,
                #[serde(default = "default_auth_message")]
                pub auth_message: String,
                #[serde(default)]
                pub first_commands: Vec<CowStr<'static>>,
                #[serde(default)]
                pub tasks: Vec<Task>,
                #[serde(default)]
                pub reconnect:
                    #[derive(Clone, Deserialize)]
                    pub struct ReconnectConfig {
                        pub max_attempts: i64,
                        pub sleep: u64,
                    },
            },

        pub args: Option<Value>,
    }
}

impl<'a> BaseConfig for ConfigEcon<'a> {
    fn nats_config(&self) -> &NatsConfig<'_> {
        &self.nats
    }

    fn logging_config(&self) -> Option<String> {
        self.logging.clone()
    }

    async fn default_config() -> &'static str {
        include_str!("../default_config/econ.yaml")
    }

    async fn econ_connect(&self) -> anyhow::Result<Econ> {
        self.econ
            .clone()
            .econ_connect(Option::from(&self.args))
            .await
    }
}

impl EconConfig {
    pub fn get_econ_addr(&self, args: Option<&Value>) -> SocketAddr {
        let args = args.unwrap_or(&Value::Null);
        formatting::get_and_format(&self.host, args, &[])
            .to_socket_addrs()
            .expect("Error create econ address")
            .next()
            .unwrap()
    }

    pub async fn econ_connect(&self, args: Option<&Value>) -> anyhow::Result<Econ> {
        let mut econ = Econ::new();
        let max_attempts = 3;
        let mut connection_error: Option<anyhow::Error> = None;

        for attempt in 1..=max_attempts {
            match econ.connect(self.get_econ_addr(args)).await {
                Ok(_) => {
                    econ.set_auth_message(self.auth_message.clone());

                    match econ.try_auth(&self.password).await {
                        Ok(true) => return Ok(econ),
                        Ok(false) => return Err(anyhow!("Econ client is not authorized")),
                        Err(err) => {
                            connection_error = Some(anyhow!("econ.try_auth, err: {}", err));
                        }
                    }
                }
                Err(err) => {
                    connection_error = Some(anyhow!(
                        "Attempt {}: econ.connect failed, err: {}",
                        attempt,
                        err
                    ));
                }
            };
            if attempt < max_attempts {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }

        Err(connection_error
            .unwrap_or_else(|| anyhow!("Failed to connect after {} attempts", max_attempts)))
    }
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            max_attempts: 20,
            sleep: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineState {
    #[serde(skip_serializing, skip_deserializing)]
    pub index: Arc<Mutex<usize>>,
    pub commands: Vec<String>,
}

impl LineState {
    pub fn new(commands: Vec<String>) -> Self {
        Self {
            index: Arc::new(Mutex::new(0)),
            commands,
        }
    }

    pub async fn get_next_command(&self) -> String {
        if self.commands.is_empty() {
            warn!("get_next_command: No commands available");
            return String::default();
        }

        let mut index = self.index.lock().await;
        let cmd = self.commands[*index % self.commands.len()].clone();
        *index += 1;
        cmd
    }
}

fn default_auth_message() -> String {
    "Authentication successful".to_string()
}
