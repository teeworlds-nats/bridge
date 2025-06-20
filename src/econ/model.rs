use crate::model::{BaseConfig, CowString};
use crate::nats::NatsConfig;
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
                pub first_commands: Vec<CowString<'static>>,
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

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            max_attempts: 20,
            sleep: 10,
        }
    }
}

fn default_tasks_delay_sec() -> u64 {
    60
}

fn default_auth_message() -> String {
    "Authentication successful".to_string()
}
