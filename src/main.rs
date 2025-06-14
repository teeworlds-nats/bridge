use clap::{Parser, Subcommand};
use errors::ConfigError;
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::flag;
use std::process::exit;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

mod bots;
mod econ;
mod errors;
mod handler;
mod model;
mod util;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[arg(short, long, global = true, default_value = "config.yaml")]
    config: String,
    #[command(subcommand)]
    action: Actions,
}

#[derive(Subcommand)]
enum Actions {
    Econ,
    Handler,
    BotReader,
    BotWriter,
}

#[tokio::main]
async fn main() -> Result<(), ConfigError> {
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
        Actions::Econ => {
            econ::main(cli.config).await.ok();
        }
        Actions::Handler => {
            handler::main(cli.config).await.ok();
        }
        Actions::BotReader => {
            bots::reader::main(cli.config).await.ok();
        }
        Actions::BotWriter => {
            // bots::writer::main().await.ok();
            panic!("NOT IMPELENTED")
        }
    }

    println!("The program ran into the end\nThis may be due to the fact that the connection has not been established");

    Ok(())
}
