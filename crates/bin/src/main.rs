//! Command line entry point and composition root.
//!
//! This binary parses the command tree, initializes logging from the
//! environment, and is the single place where adapters and use cases are wired
//! together: `run` dispatches each command group to a handler module that
//! builds its use case from the concrete adapters.

mod audit_cmd;
mod auth_cmd;
mod cli;
mod config;
mod credential_trust_cmd;
mod database_args;
mod database_cmd;
mod handlers;
mod package_cmd;
mod pki_cmd;
mod serve_alert_composition;
mod serve_alert_config;
mod serve_alert_runtime;
mod serve_alert_supervisor;
mod serve_args;
mod serve_cmd;
mod serve_deadline_runtime;
mod serve_resource_activities;
mod serve_runtime;
mod serve_signals;
mod serve_start;
mod serve_stop;
mod sign_cmd;
mod telemetry;
mod timestamp_cmd;
mod vault_cmd;
mod verify_cmd;

use clap::Parser;

use crate::cli::{Cli, Command, CryptoAction};
use crate::config::AppConfig;

fn main() -> anyhow::Result<()> {
    if let Some(code) = infrastructure::document_formats::worker_entry() {
        std::process::exit(code);
    }
    dotenvy::dotenv().ok();
    let settings = AppConfig::load();
    telemetry::init(&settings.rust_log);
    tracing::debug!(
        timestamp_authority = settings.cincel_base_url.is_some(),
        database = settings.database_url.is_some(),
        redis = settings.redis_url.is_some(),
        "settings loaded"
    );

    let cli = Cli::parse();
    run(cli)
}

fn run(cli: Cli) -> anyhow::Result<()> {
    let name = cli.command.name();
    tracing::info!(command = name, json = cli.json, "command received");
    match cli.command {
        Command::Serve(args) => serve_cmd::run(&args),
        Command::Database { action } => database_cmd::run(action, cli.json),
        Command::CredentialTrust { action } => credential_trust_cmd::run(action, cli.json),
        Command::Crypto {
            action: CryptoAction::Hash { file },
        } => handlers::hash_file(&file, cli.json),
        Command::Vault { action } => vault_cmd::run(action, cli.json),
        Command::Pki {
            scripts_dir,
            action,
        } => pki_cmd::run(&scripts_dir, action, cli.json),
        Command::Sign(args) => sign_cmd::run(&args, cli.json),
        Command::Timestamp(args) => timestamp_cmd::run(&args, cli.json),
        Command::Verify(args) => verify_cmd::run(&args, cli.json),
        Command::Package { action } => package_cmd::run(action, cli.json),
        Command::Auth { action } => auth_cmd::run(action, cli.json),
        Command::Audit { action } => audit_cmd::run(action, cli.json),
    }
}
