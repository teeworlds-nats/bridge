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
mod tg;
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
enum TgAction {
    #[command(
        about = "tg -> nats",
        long_about = "Sends all messages from telegram to the \"econ\" part",
        visible_alias = "w"
    )]
    Writer,
    #[command(
        about = "nats -> tg",
        long_about = "Received a message from \"econ\" and send it to tg",
        visible_alias = "r"
    )]
    Reader,
}

#[derive(Subcommand, Debug)]
enum Actions {
    #[command(
        about = "econ -> nats",
        visible_alias = "r"
    )]
    Econ,
    #[command(
        about = "nats -> nats",
        visible_alias = "h"
    )]
    Handler,
    #[command(
        about = "Sending-receiving messages via telegram bots",
    )]
    Tg {
        #[command(subcommand)]
        action: TgAction,
    },
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
        Actions::Tg { action } => match action {
            TgAction::Writer => tg::writer::main(cli.config).await,
            TgAction::Reader => tg::reader::main(cli.config).await,
        },
    }?;

    warn!("The program ran into the end");

    Ok(())
}
