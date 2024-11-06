use std::option::Option;
use std::error::Error;
use std::net::{SocketAddr, ToSocketAddrs};
use std::process::exit;
use serde_derive::{Deserialize, Serialize};
use async_nats::{Client, ConnectOptions, Error as NatsError};
use log::{debug, error};
use regex::Regex;
use nestify::nest;
use crate::util::utils::read_yaml_file;


#[derive(Debug, Serialize, Deserialize)]
pub struct MsgUtil {
    pub server_name: String,
    pub rcon: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MsgHandler {
    pub server_name: Option<String>,
    pub name: Option<String>,
    pub message_thread_id: String,
    pub regex_type: String,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsgBridge {
    pub server_name: String,
    pub message_thread_id: String,
    pub text: String,
}

pub struct RegexModel {
    pub name: String,
    pub regex: Regex,
    pub template: String,
}

impl RegexModel {
    pub fn new(name: &str, regex: &str, template_: Option<&str>) -> Self {
        RegexModel {
            name: name.to_string(),
            regex: Regex::new(regex).unwrap(),
            template: template_.unwrap_or_default().to_string(),
        }
    }
}

#[derive(Default)]
pub struct RconData {
    pub text: Option<String>,
    pub sync: bool,
    pub log: bool
}


pub struct EnvHandler {
    pub text: String,
    pub text_leave: String,
    pub text_join: String,
    pub nickname_regex: Vec<(Regex, String)>,
    pub block_text_in_nickname: Vec<(String, String)>,
    pub chat_regex: Vec<(Regex, String)>,
    pub block_text_in_chat: Vec<(String, String)>,
}

nest! {
    #[derive(Clone, Deserialize)]
    pub struct Env {
        pub nats:
            #[derive(Clone, Deserialize)]
            pub struct EnvNats {
                pub server: String,
                pub user: Option<String>,
                pub password: Option<String>,
            },

        // bridge

        pub check_status_econ: Option<u64>, // In Sec
        pub message_thread_id: Option<String>,
        pub server_name: Option<String>,
        pub econ: Option<
            #[derive(Clone, Deserialize)]
            pub struct EnvEcon {
                pub host: Option<String>,
                pub password: Option<String>,
                pub auth_message: Option<String>,
            }>,

        // handler
        pub text: Option<String>,
        pub text_leave: Option<String>,
        pub text_join: Option<String>,
        pub nickname_regex: Option<Vec<(String, String)>>,
        pub block_text_in_nickname: Option<Vec<(String, String)>>,
        pub chat_regex: Option<Vec<(String, String)>>,
        pub block_text_in_chat: Option<Vec<(String, String)>>,

        // util-handler
        pub commands: Option<
            #[derive(Clone, Deserialize)]
            pub struct Commands {
                sync: Option<Vec<String>>,
                log: Option<Vec<String>>
            }>
    }
}



impl Env {
    pub fn get_yaml() -> Result<Self, Box<dyn Error>> {
        debug!("Creating a structure from yaml");
        read_yaml_file("config.yaml")
    }

    pub fn get_env_handler(&self) -> Result<EnvHandler, Box<dyn Error>> {
        Ok(EnvHandler {
            text: self.text.clone().unwrap_or_else(|| "{{player}} {{text}}".to_string()),
            text_leave: self.text_leave.clone().unwrap_or_else(|| "has left the game".to_string()),
            text_join: self.text_join.clone().unwrap_or_else(|| "has join the game".to_string()),
            nickname_regex: self.nickname_regex.clone()
                .unwrap_or_default()
                .iter()
                .filter_map(|(k, v)| {
                    Regex::new(k).ok().map(|regex| (regex, v.clone())) // Клонируем v для использования в кортежах
                })
                .collect(),
            block_text_in_nickname: self.block_text_in_nickname.clone()
                .unwrap_or_else(|| vec!(("tw/".to_string(), "".to_string()), ("twitch.tv/".to_string(), "".to_string())))
                .into_iter()
                .collect(),
            chat_regex: self.chat_regex.clone()
                .unwrap_or_default()
                .iter()
                .filter_map(|(k, v)| {
                    Regex::new(k).ok().map(|regex| (regex, v.clone())) // Клонируем v для использования в кортежах
                })
                .collect(),
            block_text_in_chat: self.block_text_in_chat.clone()
                .unwrap_or_default()
                .into_iter()
                .collect()
        })

    }

    pub fn get_commands(&self) -> (Vec<String>, Vec<String>) {
        let default_sync_commands = vec![
            "ban_range".to_string(),
            "ban".to_string(),
            "unban_range".to_string(),
            "unban".to_string(),
            "muteip".to_string(),
        ];

        let default_log_commands = vec![
            "ban".to_string(),
            "ban_range".to_string(),
            "unban".to_string(),
            "unban_range".to_string(),
            "kick".to_string(),
            "muteid".to_string(),
            "muteip".to_string(),
        ];

        let sync_commands = self.commands.as_ref()
            .and_then(|commands| commands.sync.as_ref())
            .cloned()
            .unwrap_or(default_sync_commands);

        let log_commands = self.commands.as_ref()
            .and_then(|commands| commands.log.as_ref())
            .cloned()
            .unwrap_or(default_log_commands);

        (sync_commands, log_commands)
    }

    pub fn get_econ_addr(&self) -> SocketAddr {
        let Some(econ) = self.econ.clone() else {exit(-1)};
        if econ.host.is_none() {
            error!("econ.host must be set");
            exit(1);
        }
        econ.host.unwrap().to_socket_addrs().expect("Error create econ address").next().unwrap()
    }

    pub async fn connect_nats(&self) -> Result<Client, NatsError> {
        let connect = match (self.nats.user.clone(), self.nats.password.clone()) {
            (Some(user), Some(password)) => {
                ConnectOptions::new().user_and_password(user, password)
            },
            _ => {
                ConnectOptions::new()
            }
        };
        let nc = connect
            .ping_interval(std::time::Duration::from_secs(15))
            .connect(&self.nats.server)
            .await?;
        debug!("Connected nats: {}", self.nats.server);
        Ok(nc)
    }
}