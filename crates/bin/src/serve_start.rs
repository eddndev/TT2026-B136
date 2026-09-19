//! Own synchronous adapters outside the asynchronous server lifetime.

use crate::{
    serve_alert_composition::AlertConsumers,
    serve_alert_runtime, serve_alert_supervisor,
    serve_deadline_runtime::{self, DeadlineRuntimeConfig},
    serve_runtime,
    serve_signals::Signals,
    serve_stop::Stop,
};
use anyhow::Context;
use application::{deadline_dispatch::DeadlineDispatchStore, deadline_worker::DeadlineWorkerStore};
use std::sync::Arc;

pub(crate) fn run<D, W>(
    bind: &str,
    router: axum::Router,
    dispatch: D,
    worker: W,
    config: DeadlineRuntimeConfig,
    alerts: AlertConsumers,
) -> anyhow::Result<()>
where
    D: DeadlineDispatchStore + 'static,
    W: DeadlineWorkerStore + 'static,
{
    // Keep these owners outside block_on: synchronous PostgreSQL drops may block.
    let dispatch = Arc::new(dispatch);
    let worker = Arc::new(worker);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("cannot initialize async runtime")?;
    runtime.block_on(async {
        let signals = Signals::register().context("cannot register HTTP shutdown signals")?;
        let listener = tokio::net::TcpListener::bind(bind)
            .await
            .with_context(|| format!("cannot bind http server to {bind}"))?;
        let address = listener
            .local_addr()
            .context("cannot read local http address")?;
        let stop = Arc::new(Stop::default());
        let consumer_stop = Arc::clone(&stop);
        let dispatch = Arc::clone(&dispatch);
        let worker = Arc::clone(&worker);
        let deadlines = tokio::task::spawn_blocking(move || {
            serve_deadline_runtime::run(
                dispatch.as_ref(),
                worker.as_ref(),
                config,
                consumer_stop.as_ref(),
            )
        });
        let alert_stop = Arc::clone(&stop);
        let scheduler = Arc::clone(&alerts.scheduler);
        let delivery = Arc::clone(&alerts.delivery);
        let sender = alerts.sender.clone();
        let alert_config = alerts.config;
        let alerts = tokio::task::spawn_blocking(move || {
            serve_alert_runtime::run(
                scheduler.as_ref(),
                delivery.as_ref(),
                sender.as_deref(),
                alert_config,
                alert_stop.as_ref(),
            )
        });
        let consumer = tokio::spawn(serve_alert_supervisor::supervise(
            deadlines,
            alerts,
            Arc::clone(&stop),
        ));
        let (http_shutdown, shutdown) = tokio::sync::oneshot::channel();
        let server_router = router.clone();
        let http = async move {
            axum::serve(listener, server_router)
                .with_graceful_shutdown(async {
                    let _ = shutdown.await;
                })
                .await
        };
        println!("listening on http://{address}");
        tracing::info!(bind = %address, "local http application started");
        serve_runtime::supervise(http, consumer, stop, http_shutdown, signals.wait()).await
    })
}

#[cfg(test)]
#[path = "serve_start_tests.rs"]
mod tests;
