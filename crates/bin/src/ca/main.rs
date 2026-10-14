//! Separate CA operator service; office routes are not composed here.
use std::path::PathBuf;

use axum::routing::get;
use clap::{Parser, Subcommand};

mod composition;
mod config;
mod enrollment;
mod executor;
mod inspection;

#[derive(Parser)]
#[command(
    name = "qadra-ca",
    about = "Qadra CA operator service",
    args_conflicts_with_subcommands = true,
    subcommand_negates_reqs = true
)]
struct Arguments {
    /// Owner-readable configuration for the separately enrolled CA operator.
    #[arg(long, required = true)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Inspect bounded public JSON material on stdin without changing CA state.
    InspectPublic,
    /// Enroll the initial CA operator using a separate owner-only setup file.
    EnrollOperator {
        #[arg(long)]
        setup_config: PathBuf,
        #[arg(long)]
        enrollment_out: PathBuf,
    },
}

fn main() -> std::process::ExitCode {
    tracing_subscriber::fmt().json().init();
    if run(Arguments::parse()).is_err() {
        // Startup errors can contain provider details. Keep service logs bounded
        // and never format configuration, passwords, MFA material or URLs.
        eprintln!("CA service could not start; check its private configuration and dependencies");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

fn run(arguments: Arguments) -> anyhow::Result<()> {
    match arguments.command {
        Some(Command::InspectPublic) => return inspection::run(),
        Some(Command::EnrollOperator {
            setup_config,
            enrollment_out,
        }) => return enrollment::run(&setup_config, &enrollment_out),
        None => {}
    }
    let mut settings = config::load(
        arguments
            .config
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("CA service settings are required"))?,
    )?;
    let bind = settings.bind;
    let operations = settings.operations.take();
    let access = composition::open(&settings)?;
    drop(settings);
    let mut router = web::ca_access_router(access.clone(), web::HttpLimits::default())
        .route("/healthz", get(|| async { "ok" }));
    if let Some(config) = operations {
        let executor = executor::NativeCaExecutor::new(
            config.python,
            config.worker,
            config.root,
            config.inspector,
            config.cli,
            config.scripts,
            config.expected_root,
        )?;
        let operations = std::sync::Arc::new(application::ca_operations::CaOperations::new(
            access,
            std::sync::Arc::new(executor),
        ));
        router = router.merge(web::ca_operations::ca_operations_router(
            operations,
            web::HttpLimits::default(),
        ));
    }
    // Keep the final router owner outside block_on: its PostgreSQL clients
    // perform synchronous cleanup when their last shared owner is released.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let listener = tokio::net::TcpListener::bind(bind).await?;
        axum::serve(listener, router.clone())
            .with_graceful_shutdown(shutdown())
            .await?;
        Ok(())
    })
}

async fn shutdown() {
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("Unix service signal handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {},
        _ = terminate.recv() => {},
    }
}
