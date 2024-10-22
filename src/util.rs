use std::process::exit;
use std::sync::Arc;
use log::error;
use tokio::sync::Mutex;
use tw_econ::Econ;
use crate::model::Env;

pub async fn econ_connect(env: Env) -> std::io::Result<Arc<Mutex<Econ>>> {
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