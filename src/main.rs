use std::process::exit;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::flag;
use clap::{Parser, Subcommand};
use tokio::time::sleep;
use crate::model::Env;
use crate::util::errors::ConfigError;

// Actions
mod econ;
mod handler;

#[path= "util-handler/mod.rs"]
mod util_handler;

// Other
mod model;
mod util;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    action: Actions,
}

#[derive(Subcommand)]
enum Actions {
    Econ,
    Handler,
    #[clap(name = "util-handler")]
    UtilHandler
}

#[tokio::main]
async fn main() -> Result<(), ConfigError> {
    let cli = Cli::parse();

    let env = Env::get_yaml().await?;
    env_logger::init();

    let term_now = Arc::new(AtomicBool::new(false));

    for sig in TERM_SIGNALS {
        flag::register_conditional_shutdown(*sig, 1, Arc::clone(&term_now))?;
        flag::register(*sig, Arc::clone(&term_now))?;
    }

    tokio::spawn( async move {
        while !term_now.load(Ordering::Relaxed) {
            sleep(Duration::from_secs(1)).await;
        }
        exit(1);
    });

    let nc = env.connect_nats().await.unwrap();
    let js = async_nats::jetstream::new(nc.clone());

    match &cli.action {
        Actions::Econ => { econ::main(env, nc, js).await.ok(); }
        Actions::Handler => { handler::main(env.get_env_handler().unwrap(), nc, js).await.ok(); }
        Actions::UtilHandler => { util_handler::main(env, nc, js).await.ok(); }
    }
    println!("The program ran into the end");
    Ok(())
}