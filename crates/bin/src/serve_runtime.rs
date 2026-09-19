//! Supervision of HTTP drainage and the blocking deadline consumer.

use crate::serve_stop::Stop;
use application::ApplicationError;
use std::{future::Future, io, sync::Arc};
use tokio::{
    sync::oneshot,
    task::{JoinError, JoinHandle},
};

type ConsumerResult = Result<Result<(), ApplicationError>, JoinError>;

enum Exit {
    Signal(io::Result<ShutdownSignal>),
    Http(io::Result<()>),
    Consumer(ConsumerResult),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShutdownSignal {
    Interrupt,
    Terminate,
}

pub(crate) async fn supervise<H, S>(
    http: H,
    mut consumer: JoinHandle<Result<(), ApplicationError>>,
    stop: Arc<Stop>,
    http_shutdown: oneshot::Sender<()>,
    signal: S,
) -> anyhow::Result<()>
where
    H: Future<Output = io::Result<()>>,
    S: Future<Output = io::Result<ShutdownSignal>>,
{
    tokio::pin!(http, signal);
    let exit = tokio::select! {
        value = &mut signal => Exit::Signal(value),
        value = &mut http => Exit::Http(value),
        value = &mut consumer => Exit::Consumer(value),
    };
    stop.request();
    let _ = http_shutdown.send(());
    let mut failures = Vec::new();
    let (http_result, consumer_result) = match exit {
        Exit::Signal(signal) => {
            if signal.is_err() {
                failures.push("signal_failed");
            }
            tokio::join!(http, consumer)
        }
        Exit::Http(result) => {
            if result.is_ok() {
                failures.push("http_stopped");
            }
            (result, consumer.await)
        }
        Exit::Consumer(result) => {
            if matches!(result, Ok(Ok(()))) {
                failures.push("consumer_stopped");
            }
            (http.await, result)
        }
    };
    if http_result.is_err() {
        failures.push("http_failed");
    }
    match consumer_result {
        Ok(Ok(())) => {}
        Ok(Err(_)) => failures.push("consumer_failed"),
        Err(_) => failures.push("consumer_panicked"),
    }
    if failures.is_empty() {
        Ok(())
    } else {
        let failures = failures.join(",");
        tracing::error!(failures, "HTTP application stopped after drainage");
        Err(anyhow::anyhow!("HTTP application stopped: {failures}"))
    }
}

#[cfg(test)]
#[path = "serve_runtime_support.rs"]
mod support;
#[cfg(test)]
#[path = "serve_runtime_tests.rs"]
mod tests;
