use std::process::exit;
use std::sync::Arc;
use std::time::Duration;
use futures_util::StreamExt;
use log::{debug};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tw_econ::Econ;
use crate::util::utils::err_to_string_and_exit;

pub async fn sender_message_to_tw(nc: async_nats::Client, message_thread_id: String, econ: Arc<Mutex<Econ>>) {
    let mut subscriber = nc.queue_subscribe(
        format!("tw.econ.write.{}", message_thread_id),
        format!("bridge_.{}", message_thread_id)
    ).await.unwrap();

    while let Some(message) = subscriber.next().await {
        let msg: &str = match std::str::from_utf8(&message.payload) {
            Ok(json_string) => json_string,
            Err(err) => {
                eprintln!("Error converting bytes to string: {}", err);
                exit(0);
            }
        };

        let mut econ_lock = econ.lock().await;
        match econ_lock.send_line(msg) {
            Ok(_) => {}
            Err(err) => { err_to_string_and_exit("Error send_line to econ: ", Box::new(err)) }
        };
    }
}


pub async fn moderator_tw(econ: Arc<Mutex<Econ>>, nc: async_nats::Client) {
    let mut subscriber = nc.subscribe("tw.econ.moderator").await.unwrap();

    while let Some(message) = subscriber.next().await {
        let msg: &str = std::str::from_utf8(&message.payload).unwrap_or_else(|err| {
            eprintln!("Error converting bytes to string: {}", err);
            exit(0);
        });

        debug!("send_line to econ: {}", msg);
        let mut econ_lock = econ.lock().await;
        match econ_lock.send_line(msg) {
            Ok(_) => {}
            Err(err) => { err_to_string_and_exit("Error send_line to econ: ", Box::new(err)) }
        };
    }
}

pub async fn check_status(econ: Arc<Mutex<Econ>>, check_status_econ_sleep: Option<u64>) {
    let check_status_econ_sleep = check_status_econ_sleep.unwrap_or(15);
    loop {
        debug!("check status econ");
        let mut econ_lock = econ.lock().await;
        match econ_lock.send_line("") {
            Ok(_) => {}
            Err(err) => { err_to_string_and_exit("Error send_line to econ: ", Box::new(err)) }
        };
        sleep(Duration::from_secs(check_status_econ_sleep)).await;
    }
}
