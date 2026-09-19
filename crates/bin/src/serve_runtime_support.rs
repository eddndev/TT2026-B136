use super::{supervise, ShutdownSignal};
use crate::{serve_deadline_runtime::RuntimeControl, serve_stop::Stop};
use application::ApplicationError;
use std::{
    future::Future,
    io,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering::SeqCst},
        mpsc, Arc,
    },
    task::{Context, Poll, Waker},
    time::Duration,
};
use tokio::{sync::oneshot, task::AbortHandle};

pub const WATCHDOG: Duration = Duration::from_secs(3);
const PRIVATE_HTTP: &str = "private-http-native-message";
const PRIVATE_CONSUMER: &str = "private-consumer-native-message";
const PRIVATE_SIGNAL: &str = "private-signal-native-message";

#[derive(Clone, Copy)]
pub enum ConsumerExit {
    Success,
    Error,
    Panic,
}

type Supervisor = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>>;

pub struct Running {
    supervisor: Supervisor,
    result: Option<anyhow::Result<()>>,
    pub stop: Arc<Stop>,
    signal: Option<oneshot::Sender<io::Result<ShutdownSignal>>>,
    http_release: Option<oneshot::Sender<io::Result<()>>>,
    consumer_release: Option<mpsc::SyncSender<()>>,
    consumer_handle: AbortHandle,
    http_shutdown_seen: Arc<AtomicBool>,
    http_finished: Arc<AtomicBool>,
    consumer_finished: Arc<AtomicBool>,
}

impl Running {
    pub async fn start(exit: ConsumerExit) -> Self {
        let stop = Arc::new(Stop::default());
        let (entered_tx, entered) = oneshot::channel();
        let (consumer_release, release) = mpsc::sync_channel(1);
        let consumer_finished = Arc::new(AtomicBool::new(false));
        let finished = Arc::clone(&consumer_finished);
        let consumer = tokio::task::spawn_blocking(move || {
            let _ = entered_tx.send(());
            release
                .recv_timeout(WATCHDOG)
                .expect("consumer release watchdog");
            finished.store(true, SeqCst);
            match exit {
                ConsumerExit::Success => Ok(()),
                ConsumerExit::Error => Err(ApplicationError::Port(PRIVATE_CONSUMER.into())),
                ConsumerExit::Panic => panic!("private-consumer-native-message"),
            }
        });
        tokio::time::timeout(WATCHDOG, entered)
            .await
            .expect("consumer entry watchdog")
            .unwrap();
        let consumer_handle = consumer.abort_handle();

        let (http_shutdown, mut shutdown) = oneshot::channel();
        let (http_release, mut release) = oneshot::channel();
        let http_shutdown_seen = Arc::new(AtomicBool::new(false));
        let seen = Arc::clone(&http_shutdown_seen);
        let http_finished = Arc::new(AtomicBool::new(false));
        let finished = Arc::clone(&http_finished);
        let http = async move {
            let value = tokio::select! {
                notified = &mut shutdown => {
                    notified.map_err(|_| io::Error::other("shutdown sender dropped"))?;
                    seen.store(true, SeqCst);
                    release.await
                },
                value = &mut release => value,
            };
            finished.store(true, SeqCst);
            value.map_err(|_| io::Error::other("HTTP release sender dropped"))?
        };
        let (signal, signal_rx) = oneshot::channel();
        let signal_future = async move {
            signal_rx
                .await
                .map_err(|_| io::Error::other(PRIVATE_SIGNAL))?
        };
        Self {
            supervisor: Box::pin(supervise(
                http,
                consumer,
                Arc::clone(&stop),
                http_shutdown,
                signal_future,
            )),
            result: None,
            stop,
            signal: Some(signal),
            http_release: Some(http_release),
            consumer_release: Some(consumer_release),
            consumer_handle,
            http_shutdown_seen,
            http_finished,
            consumer_finished,
        }
    }

    /// Drive the actual supervisor once with all currently-ready events.
    pub fn pending(&mut self) -> bool {
        if self.result.is_some() {
            return false;
        }
        let mut context = Context::from_waker(Waker::noop());
        if let Poll::Ready(value) = self.supervisor.as_mut().poll(&mut context) {
            self.result = Some(value);
        }
        self.result.is_none()
    }

    pub fn signal(&mut self, signal: ShutdownSignal) {
        let _ = self.signal.take().unwrap().send(Ok(signal));
    }

    pub fn signal_error(&mut self) {
        let _ = self
            .signal
            .take()
            .unwrap()
            .send(Err(io::Error::other(PRIVATE_SIGNAL)));
    }

    pub fn release_http(&mut self, failed: bool) {
        let outcome = if failed {
            Err(io::Error::other(PRIVATE_HTTP))
        } else {
            Ok(())
        };
        let _ = self.http_release.take().unwrap().send(outcome);
    }

    pub fn release_consumer(&mut self) {
        let _ = self.consumer_release.take().unwrap().send(());
    }

    pub async fn consumer_done(&self) {
        tokio::time::timeout(WATCHDOG, async {
            while !self.consumer_handle.is_finished() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("consumer completion watchdog");
    }

    pub fn shutdown_seen(&self) -> bool {
        self.http_shutdown_seen.load(SeqCst)
    }

    pub fn stopped(&self) -> bool {
        self.stop.is_stopped()
    }

    pub fn both_finished(&self) -> bool {
        self.http_finished.load(SeqCst) && self.consumer_finished.load(SeqCst)
    }

    pub fn http_finished(&self) -> bool {
        self.http_finished.load(SeqCst)
    }

    pub async fn finish(mut self) -> anyhow::Result<()> {
        if let Some(result) = self.result.take() {
            return result;
        }
        tokio::time::timeout(WATCHDOG, self.supervisor)
            .await
            .expect("supervisor completion watchdog")
    }
}

pub fn assert_codes(result: anyhow::Result<()>, codes: &[&str]) {
    let error = result.expect_err("supervision failure must not become success");
    let message = format!("{error:#}");
    for code in codes {
        assert!(message.contains(code), "missing {code}: {message}");
    }
    for private in [PRIVATE_HTTP, PRIVATE_CONSUMER, PRIVATE_SIGNAL] {
        assert!(
            !message.contains(private),
            "native diagnostic escaped safe supervision error"
        );
    }
}
