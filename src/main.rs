mod model;

use std::process::exit;
use std::sync::Arc;
use std::time::Duration;
use log::{debug, error, info};
use tw_econ::Econ;
use futures_util::stream::StreamExt;
use tokio::sync::Mutex;
use tokio::time::sleep;
use crate::model::{Env, Msg, MsgHandler};


#[tokio::main]
async fn main() -> Result<(), async_nats::Error>  {
    let env = match Env::get_yaml() {
        Ok(env) => {env}
        Err(err) => {eprintln!("Failed open yaml fail: {}", err); exit(0)}
    };
    env_logger::init();

    let nc = env.connect_nats().await?;
    let js = async_nats::jetstream::new(nc.clone());
    let econ_read = econ_connect(env.clone()).await?;
    let econ_write = econ_connect(env.clone()).await?;
    info!("econ connected");


    tokio::spawn(sender_message_to_tw(nc.clone(), js.clone(), env.clone(), econ_write.clone()));

    tokio::spawn(moderator_tw(econ_write.clone(), nc.clone()));

    loop {
        let Some(message) = (match econ_read.lock().await.recv_line(true) {
            Ok(result) => { result }
            Err(err) => {
                error!("err to loop: {}", err);
                tokio::time::sleep(Duration::from_millis(50)).await;
                continue
            }
        }) else { continue};

        let send_msg = Msg {
            server_name: env.server_name.clone(),
            message_thread_id: env.message_thread_id.clone(),
            text: message.clone()
        };
        let json = match serde_json::to_string_pretty(&send_msg) {
            Ok(str) => {str}
            Err(err) => {error!("Json Serialize Error: {}", err); String::new()}
        };

        debug!("sended json to tw.handler: {}", json);
        js.publish("tw.handler", json.into())
            .await
            .expect("Error publish message to tw.handler");
    }
}

async fn sender_message_to_tw(nc: async_nats::Client, js: async_nats::jetstream::Context, env: Env, econ: Arc<Mutex<Econ>>) {
    let mut subscriber = nc.queue_subscribe(
        format!("tw.{}", env.message_thread_id),
        format!("bridge_.{}", env.message_thread_id)
    ).await.unwrap();

    let addr = env.get_econ_addr();

    while let Some(message) = subscriber.next().await {
        let msg: &str = match std::str::from_utf8(&message.payload) {
            Ok(json_string) => json_string,
            Err(err) => {
                error!("Error converting bytes to string: {}", err);
                continue;
            }
        };

        if msg == "get addr" {
            let send_msg = MsgHandler {
                data: None,
                server_name: "".to_string(),
                name: None,
                message_thread_id: env.message_thread_id.clone(),
                regex_type: "addr".to_string(),
                text: addr.to_string(),
            };

            let json = match serde_json::to_string_pretty(&send_msg) {
                Ok(str) => str,
                Err(err) => {
                    error!("Json Serialize Error: {}", err);
                    continue;
                }
            };

            debug!("sended json to tw.messages: {}", json);
            js.publish("tw.messages", json.into())
                .await
                .expect("Error publish message to tw.messages");
            continue;
        }

        let mut econ_lock = econ.lock().await;
        match econ_lock.send_line(msg) {
            Ok(_) => {}
            Err(err) => {
                error!("Error send_line to econ: {}", err);
                continue;
            }
        };
    }
}


async fn moderator_tw(econ: Arc<Mutex<Econ>>, nc: async_nats::Client) {
    let mut subscriber = nc.subscribe("tw.moderator").await.unwrap();

    while let Some(message) = subscriber.next().await {
        let msg: &str = match std::str::from_utf8(&message.payload) {
            Ok(json_string) => json_string,
            Err(err) => {
                error!("Error converting bytes to string: {}", err);
                continue
            }
        };

        debug!("send_line to econ: {}", msg);
        let mut econ_lock = econ.lock().await;
        match econ_lock.send_line(msg) {
            Ok(_) => {}
            Err(err) => {
                error!("Error send_line to econ: {}", err);
                continue
            }
        };
    }
    sleep(Duration::from_millis(50)).await;
}


async fn econ_connect(env: Env) -> std::io::Result<Arc<Mutex<Econ>>> {
    let econ = Arc::new(Mutex::new(Econ::new()));

    {
        let mut econ_lock = econ.lock().await;
        econ_lock.connect(env.get_econ_addr())?;

        if let Some(msg) = env.auth_message {
            econ_lock.set_auth_message(msg);
        }

        let authed = econ_lock.try_auth(env.econ_password)?;
        if !authed {
            error!("Econ client not authed");
            exit(0); // Завершаем процесс
        }
    }

    Ok(econ)
}

