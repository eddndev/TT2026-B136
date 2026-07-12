//! Command line entry point and composition root.
//!
//! This binary parses the command tree, initializes logging from the
//! environment, and is the single place where adapters and use cases are wired
//! together. Wiring is added as each use case lands; for now the tree parses
//! and reports which command was requested.

mod cli;
mod config;
mod telemetry;

use clap::Parser;

use crate::cli::Cli;
use crate::config::AppConfig;

fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let settings = AppConfig::load();
    telemetry::init(&settings.rust_log);
    tracing::debug!(
        timestamp_authority = settings.cincel_base_url.is_some(),
        database = settings.database_url.is_some(),
        "settings loaded"
    );

    let cli = Cli::parse();
    run(cli)
}

fn run(cli: Cli) -> anyhow::Result<()> {
    let name = cli.command.name();
    tracing::info!(command = name, json = cli.json, "command received");
    anyhow::bail!("command '{name}' is not implemented yet")
}
