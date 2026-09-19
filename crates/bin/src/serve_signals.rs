//! Register process shutdown signals before the HTTP application starts work.

use crate::serve_runtime::ShutdownSignal;
use std::io;

#[cfg(unix)]
pub(crate) struct Signals {
    interrupt: tokio::signal::unix::Signal,
    terminate: tokio::signal::unix::Signal,
}

#[cfg(unix)]
impl Signals {
    pub(crate) fn register() -> io::Result<Self> {
        use tokio::signal::unix::{signal, SignalKind};
        Ok(Self {
            interrupt: signal(SignalKind::interrupt())?,
            terminate: signal(SignalKind::terminate())?,
        })
    }

    pub(crate) async fn wait(mut self) -> io::Result<ShutdownSignal> {
        tokio::select! {
            value = self.interrupt.recv() => received(value, ShutdownSignal::Interrupt),
            value = self.terminate.recv() => received(value, ShutdownSignal::Terminate),
        }
    }
}

#[cfg(unix)]
fn received(value: Option<()>, signal: ShutdownSignal) -> io::Result<ShutdownSignal> {
    value
        .map(|()| signal)
        .ok_or_else(|| io::Error::other("shutdown signal stream closed"))
}

#[cfg(windows)]
pub(crate) struct Signals {
    interrupt: tokio::signal::windows::CtrlC,
}

#[cfg(windows)]
impl Signals {
    pub(crate) fn register() -> io::Result<Self> {
        Ok(Self {
            interrupt: tokio::signal::windows::ctrl_c()?,
        })
    }

    pub(crate) async fn wait(mut self) -> io::Result<ShutdownSignal> {
        self.interrupt
            .recv()
            .await
            .map(|()| ShutdownSignal::Interrupt)
            .ok_or_else(|| io::Error::other("shutdown signal stream closed"))
    }
}
