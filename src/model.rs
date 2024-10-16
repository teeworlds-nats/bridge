use std::option::Option;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::net::{SocketAddr, ToSocketAddrs};
use async_nats::{Client, ConnectOptions, Error as NatsError};
use log::debug;
use serde_derive::{Deserialize, Serialize};


fn read_yaml_file(file_path: &str) -> Result<Env, Box<dyn Error>> {
    let mut file = File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let env: Env = serde_yaml::from_str(&contents)?;
    Ok(env)
}


#[derive(Debug, Serialize)]
pub struct Msg {
    pub server_name: String,
    pub message_thread_id: String,
    pub text: String,
}


#[derive(Debug, Clone, Deserialize)]
pub struct Env {
    pub message_thread_id: String,
    pub server_name: String,
    pub nats_server: String,
    pub nats_user: Option<String>,
    pub nats_password: Option<String>,
    pub econ_host: String,
    pub econ_port: u16,
    pub econ_password: String,
    pub auth_message: Option<String>,
}


impl Env {
    pub fn get_yaml() -> Result<Self, Box<dyn Error>> {
        debug!("Creating a structure from yaml");
        read_yaml_file("config.yaml")
    }

    pub fn get_econ_addr(&self) -> SocketAddr {
        format!("{}:{}", self.econ_host, self.econ_port).to_socket_addrs().expect("Error create econ address").next().unwrap()
    }

    pub async fn connect_nats(&self) -> Result<Client, NatsError> {
        let nats_user = self.nats_user.clone();
        let nats_password = self.nats_password.clone();

        let connect = match (nats_user, nats_password) {
            (Some(user), Some(password)) => {
                debug!("Connected nats from user and password: {}", self.nats_server);
                ConnectOptions::new().user_and_password(user, password)
            },
            _ => {
                debug!("Connected nats: {}", self.nats_server);
                ConnectOptions::new()
            }
        };

        let nc = connect
            .ping_interval(std::time::Duration::from_secs(15))
            .connect(&self.nats_server)
            .await?;
        Ok(nc)
    }
}