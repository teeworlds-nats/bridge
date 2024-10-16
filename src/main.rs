mod model;

use std::process::exit;
use std::time::Duration;
use log::{debug, error, info};
use tw_econ::Econ;
use futures_util::stream::StreamExt;
use crate::model::{Env, Msg};

async fn econ_connect(env: Env) -> std::io::Result<Econ> {
    let mut econ = Econ::new();
    econ.connect(env.get_econ_addr())?;

    if let Some(msg) = env.auth_message {econ.set_auth_message(msg)}

    let authed = econ.try_auth(env.econ_password)?;
    if !authed {
        error!("Econ client not authed"); exit(0)
    }

    Ok(econ)
}

async fn sender_message_to_tw(mut econ: Econ, message_thread_id: String, nc: async_nats::Client) {
    let mut subscriber = nc.queue_subscribe(
        format!("teesports.{}", message_thread_id),
        format!("bridge_.{}", message_thread_id)
    ).await.unwrap();

    while let Some(message) = subscriber.next().await {
        let msg: &str = match std::str::from_utf8(&message.payload) {
            Ok(json_string) => json_string,
            Err(err) => { error!("Error converting bytes to string: {}", err); continue }
        };

        match econ.send_line(format!("say \"{}\"", msg)) {
            Ok(_) => {}
            Err(err) => { error!("Error send_line to econ: {}", err); continue }
        };
    }
}


#[tokio::main]
async fn main() -> Result<(), async_nats::Error>  {
    let env = match Env::get_yaml() {
        Ok(env) => {env}
        Err(err) => {eprintln!("Failed open yaml fail: {}", err); exit(0)}
    };
    env_logger::init();

    let nc = env.connect_nats().await?;
    let js = async_nats::jetstream::new(nc.clone());

    let mut econ = econ_connect(env.clone()).await.expect("Failed connect econ");
    info!("econ connected");

    tokio::spawn(sender_message_to_tw(
        econ_connect(env.clone()).await.expect("Failed connect econ"),
        env.message_thread_id.clone(),
        nc.clone()
    ));

    loop {
        let Some(message) = (match econ.recv_line(true) {
            Ok(result) => { result }
            Err(err) => {
                error!("err to loop: {}", err);
                tokio::time::sleep(Duration::from_millis(50)).await;
                continue
            }
        }) else { continue };

        let send_msg = Msg {
            server_name: env.server_name.clone(),
            message_thread_id: env.message_thread_id.clone(),
            text: message.clone()
        };

        let json = match serde_json::to_string_pretty(&send_msg) {
            Ok(str) => {str}
            Err(err) => {error!("Json Serialize Error: {}", err); continue}
        };

        debug!("sended json to teesports.handler: {}", json);
        js.publish("teesports.handler", json.into())
            .await
            .expect("Error publish message to teesports.messages");
    }
}