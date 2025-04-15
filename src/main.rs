use crate::model::Config;
use clap::{Parser, Subcommand};
use errors::ConfigError;
use log::info;
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::flag;
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use teloxide::prelude::Requester;
use tokio::time::sleep;

// Actions
mod econ;
mod handlers;

// Other
mod bots;
mod errors;
mod model;

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
    HandlerAuto,
    BotReader,
    BotWriter,
}

#[tokio::main]
async fn main() -> Result<(), ConfigError> {
    let cli = Cli::parse();

    let config = Config::get_yaml().await?;
    config.set_logging();

    let term_now = Arc::new(AtomicBool::new(false));

    for sig in TERM_SIGNALS {
        flag::register_conditional_shutdown(*sig, 1, Arc::clone(&term_now))?;
        flag::register(*sig, Arc::clone(&term_now))?;
    }

    tokio::spawn(async move {
        while !term_now.load(Ordering::Relaxed) {
            sleep(Duration::from_secs(1)).await;
        }
        exit(1);
    });

    let nc = config.connect_nats().await.unwrap();
    let js = async_nats::jetstream::new(nc.clone());

    match &cli.action {
        Actions::Econ => {
            econ::main(config, nc, js).await.ok();
        }
        Actions::Handler => {
            handlers::handler::main(config, nc, js).await.ok();
        }
        Actions::HandlerAuto => {
            handlers::handler_auto::main(config, nc, js).await.ok();
        }
        Actions::BotReader => {
            let bot = config.connect_bot();
            let me = bot.get_me().await.expect("Failed to execute bot.get_me");

            info!("bot {} has started", me.username());
            bots::reader::main(config, nc, js, bot).await.ok();
        }
        Actions::BotWriter => {
            // handlers::handler_auto::main(config, nc, js).await.ok();
            panic!("NOT IMPELENTED")
        }
    }

    println!("The program ran into the end\nThis may be due to the fact that the connection has not been established");

    Ok(())
}
