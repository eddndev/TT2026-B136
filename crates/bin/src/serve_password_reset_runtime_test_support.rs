use crate::{
    serve_password_reset_runtime::{
        PasswordResetAdmission, PasswordResetConsumer, ResetRequestWork,
    },
    serve_stop::Stop,
};
use application::{identity::password_reset::ResetRequestAccepted, ApplicationError};
use std::{
    future::Future,
    num::NonZeroUsize,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering::SeqCst},
        mpsc, Arc, Mutex,
    },
    task::{Context, Poll, Waker},
    time::Duration,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use web::{
    password_reset::{PasswordResetRequests, RequestAdmission},
    HttpWorkBudget,
};
use zeroize::Zeroizing;

pub const WATCHDOG: Duration = Duration::from_secs(5);
pub const PRIVATE_ERROR: &str = "private-reset-driver-native-message";
pub type ConsumerFuture = Pin<Box<dyn Future<Output = Result<(), ApplicationError>> + Send>>;

#[derive(Clone, Copy)]
pub enum Outcome {
    Accepted,
    Error,
    Panic,
}

pub struct Work {
    pub handler: ResetRequestWork,
    pub calls: Arc<AtomicUsize>,
    pub entered: UnboundedReceiver<()>,
    release: mpsc::Sender<()>,
}

impl Work {
    pub fn new(outcome: Outcome) -> Self {
        let calls = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&calls);
        let (entered_tx, entered) = unbounded_channel();
        let (release, released) = mpsc::channel();
        let released = Mutex::new(released);
        let handler: ResetRequestWork = Arc::new(move |email: Zeroizing<String>| {
            assert!(email.as_str() == "fixture@example.test");
            count.fetch_add(1, SeqCst);
            entered_tx.send(()).expect("entry observer present");
            released
                .lock()
                .unwrap()
                .recv_timeout(WATCHDOG)
                .expect("work release watchdog");
            match outcome {
                Outcome::Accepted => Ok(ResetRequestAccepted),
                Outcome::Error => Err(ApplicationError::Port(PRIVATE_ERROR.into())),
                Outcome::Panic => panic!("private-reset-driver-native-message"),
            }
        });
        Self {
            handler,
            calls,
            entered,
            release,
        }
    }

    pub fn release(&self) {
        self.release.send(()).expect("work receiver present");
    }
    pub fn count(&self) -> usize {
        self.calls.load(SeqCst)
    }
}

pub struct Running {
    pub admission: PasswordResetAdmission,
    pub work: Work,
    pub budget: HttpWorkBudget,
    pub stop: Arc<Stop>,
    pub future: ConsumerFuture,
}

impl Running {
    pub fn new(slots: usize, outcome: Outcome) -> Self {
        let work = Work::new(outcome);
        let stop = Arc::new(Stop::default());
        let budget = HttpWorkBudget::new(NonZeroUsize::new(slots).unwrap());
        let (admission, consumer) = PasswordResetConsumer::new(
            Arc::clone(&work.handler),
            budget.clone(),
            Arc::clone(&stop),
        );
        Self {
            admission,
            work,
            budget,
            stop,
            future: Box::pin(consumer.run()),
        }
    }

    pub fn submit(&self) -> RequestAdmission {
        self.admission
            .try_submit(Zeroizing::new("fixture@example.test".into()))
    }

    pub fn pending(&mut self) -> bool {
        poll(self.future.as_mut()).is_pending()
    }

    pub async fn entered(&mut self) {
        let wait = async {
            tokio::select! {
                result = &mut self.future => {
                    assert!(result.is_ok(), "consumer failed before request entry");
                    panic!("consumer ended before request entry");
                }
                entry = self.work.entered.recv() => assert!(entry.is_some()),
            }
        };
        tokio::time::timeout(WATCHDOG, wait)
            .await
            .expect("entry watchdog");
    }

    pub async fn finish(self) -> Result<(), ApplicationError> {
        tokio::time::timeout(WATCHDOG, self.future)
            .await
            .expect("consumer join watchdog")
    }
}

pub fn poll<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}
