mod model;
mod handlers;
mod util;

use std::process::exit;
use log::{debug, error, info};
use crate::handlers::{moderator_tw, sender_message_to_tw};
use crate::model::{Env, Msg};
use crate::util::econ_connect;

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

    tokio::spawn(sender_message_to_tw(nc.clone(), env.message_thread_id.clone(), econ_write.clone()));
    tokio::spawn(moderator_tw(econ_write.clone(), nc.clone()));

    loop {
        let Some(message) = (match econ_read.lock().await.recv_line(true) {
            Ok(result) => { result }
            Err(err) => {
                error!("err to loop: {}", err);
                exit(0);
            }
        }) else { continue};

        let send_msg = Msg {
            server_name: env.server_name.clone(),
            message_thread_id: env.message_thread_id.clone(),
            text: message
        };
        let json = serde_json::to_string_pretty(&send_msg).expect("Failed Serialize Msg");

        debug!("send json to tw.econ.read.(id): {}", json);
        js.publish("tw.econ.read.".to_owned() + &env.message_thread_id.clone(), json.into())
            .await
            .expect("Error publish message to tw.econ.read.(id)");
    }
}

