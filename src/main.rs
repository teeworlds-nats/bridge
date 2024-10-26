use std::process::exit;
use clap::{Parser, Subcommand};
use crate::model::Env;

// Actions
mod bridge;
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
    Bridge,
    Handler,
    #[clap(name = "util-handler")]
    UtilHandler
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    let env = match Env::get_yaml() {
        Ok(env) => {env}
        Err(err) => {eprintln!("Failed open yaml fail: {}", err); exit(0)}
    };
    env_logger::init();

    let nc = env.connect_nats().await.unwrap();
    let js = async_nats::jetstream::new(nc.clone());

    match &cli.action {
        Actions::Bridge => { bridge::main(env, nc, js).await.ok(); }
        Actions::Handler => { handler::main(env.get_env_handler().unwrap(), nc, js).await.ok(); }
        Actions::UtilHandler => { util_handler::main(env, nc, js).await.ok(); }
    }

    Ok(())
}