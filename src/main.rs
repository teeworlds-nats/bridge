extern crate core;

use clap::{Parser, Subcommand};
use log::warn;
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::flag;
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

mod args;
mod bots;
mod econ;
mod errors;
mod format;
mod handler;
mod model;
mod nats;
mod util;
mod value;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[arg(short, long, global = true, default_value = "config.yaml")]
    config: String,
    #[command(subcommand)]
    action: Actions,
}

#[derive(Subcommand, Debug)]
enum Actions {
    Econ,
    Handler,
    BotReader,
    BotWriter,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

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

    match &cli.action {
        Actions::Econ => econ::main(cli.config).await,
        Actions::Handler => handler::main(cli.config).await,
        Actions::BotReader => bots::reader::main(cli.config).await,
        Actions::BotWriter => bots::writer::main(cli.config).await,
    }
    .expect("Service operation error");

    warn!("The program ran into the end");

    Ok(())
}
