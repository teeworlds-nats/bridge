use crate::model::{BaseConfig, NatsConfig};
use crate::util::get_and_format;
use anyhow::anyhow;
use nestify::nest;
use serde_derive::{Deserialize, Serialize};
use serde_yaml::Value;
use std::net::{SocketAddr, ToSocketAddrs};
use tw_econ::Econ;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsgBridge {
    pub text: String,
    pub args: Value,
}

use tokio::sync::Mutex;

#[derive(Default, Debug)]
pub struct ConnectionState {
    pub is_connecting: Mutex<bool>,
    pub pending_messages: Mutex<Vec<String>>,
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
                #[serde(default)]
                pub reconnect:
                    #[derive(Default, Clone, Deserialize)]
                    pub struct ReconnectConfig {
                        #[serde(default = "default_max_attempts")]
                        pub max_attempts: i64,
                        #[serde(default = "default_sleep")]
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
        econ.set_auth_message(self.auth_message.clone());

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

fn default_auth_message() -> String {
    "Authentication successful".to_string()
}
