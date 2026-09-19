use super::{AlertRuntimeConfig, RuntimeControl};
use application::{alerts::*, ApplicationError};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering::SeqCst},
        Arc, Mutex,
    },
    time::Duration,
};
use time::OffsetDateTime;
use uuid::Uuid;

pub struct Control {
    pub events: Mutex<Vec<&'static str>>,
    pub stopped: AtomicBool,
    pub waits: Mutex<Vec<Duration>>,
    pub cycles: usize,
    pub stop_at: Option<&'static str>,
}
impl Control {
    pub fn new(cycles: usize, stop_at: Option<&'static str>) -> Arc<Self> {
        Arc::new(Self {
            events: Mutex::new(vec![]),
            stopped: AtomicBool::new(false),
            waits: Mutex::new(vec![]),
            cycles,
            stop_at,
        })
    }
    fn record(&self, event: &'static str) {
        self.events.lock().unwrap().push(event);
        if self.stop_at == Some(event) {
            self.stopped.store(true, SeqCst);
        }
    }
    pub fn events(&self) -> Vec<&'static str> {
        self.events.lock().unwrap().clone()
    }
}
impl RuntimeControl for Control {
    fn is_stopped(&self) -> bool {
        self.stopped.load(SeqCst)
    }
    fn wait(&self, duration: Duration) {
        self.record("wait");
        let mut waits = self.waits.lock().unwrap();
        waits.push(duration);
        if waits.len() >= self.cycles {
            self.stopped.store(true, SeqCst);
        }
    }
}

pub struct Scheduler {
    pub control: Arc<Control>,
    pub replies: Mutex<VecDeque<Result<AlertSchedulerRun, ApplicationError>>>,
}
impl Scheduler {
    pub fn new(
        control: &Arc<Control>,
        replies: Vec<Result<AlertSchedulerRun, ApplicationError>>,
    ) -> Self {
        Self {
            control: control.clone(),
            replies: Mutex::new(replies.into()),
        }
    }
}
impl AlertSchedulerStore for Scheduler {
    fn run_next(&self) -> Result<AlertSchedulerRun, ApplicationError> {
        self.control.record("schedule");
        self.replies
            .lock()
            .unwrap()
            .pop_front()
            .expect("bounded scheduler calls")
    }
}
pub struct Delivery {
    pub control: Arc<Control>,
    pub claims: Mutex<VecDeque<Result<Option<AlertDeliveryClaim>, ApplicationError>>>,
    pub completions: Mutex<Vec<AlertDeliveryCompletion>>,
    pub outcomes: Mutex<VecDeque<Result<(), ApplicationError>>>,
}
impl Delivery {
    pub fn new(
        control: &Arc<Control>,
        claims: Vec<Result<Option<AlertDeliveryClaim>, ApplicationError>>,
        outcomes: Vec<Result<(), ApplicationError>>,
    ) -> Self {
        Self {
            control: control.clone(),
            claims: Mutex::new(claims.into()),
            completions: Mutex::new(vec![]),
            outcomes: Mutex::new(outcomes.into()),
        }
    }
}
impl AlertDeliveryStore for Delivery {
    fn claim_next(&self) -> Result<Option<AlertDeliveryClaim>, ApplicationError> {
        self.control.record("claim");
        self.claims
            .lock()
            .unwrap()
            .pop_front()
            .expect("bounded claims")
    }
    fn complete_attempt(
        &self,
        completion: AlertDeliveryCompletion,
    ) -> Result<(), ApplicationError> {
        self.control.record("complete");
        self.completions.lock().unwrap().push(completion);
        self.outcomes
            .lock()
            .unwrap()
            .pop_front()
            .expect("bounded completions")
    }
}
pub struct Sender {
    pub control: Arc<Control>,
    pub sent: Mutex<Vec<AlertEmailMessage>>,
    pub outcome: AlertEmailOutcome,
}
impl Sender {
    pub fn new(control: &Arc<Control>) -> Self {
        Self {
            control: control.clone(),
            sent: Mutex::new(vec![]),
            outcome: AlertEmailOutcome::Unknown {
                code: "transport_uncertain".into(),
            },
        }
    }
}
impl AlertEmailSender for Sender {
    fn send(&self, message: &AlertEmailMessage) -> AlertEmailOutcome {
        self.control.record("send");
        self.sent.lock().unwrap().push(message.clone());
        self.outcome.clone()
    }
}
pub fn config(budget: u32) -> AlertRuntimeConfig {
    AlertRuntimeConfig::new(budget, Duration::from_secs(1)).unwrap()
}
pub fn progress() -> Result<AlertSchedulerRun, ApplicationError> {
    Ok(AlertSchedulerRun::Activated {
        alert_id: AlertId::from_uuid(Uuid::from_u128(1)),
    })
}
pub fn claim() -> AlertDeliveryClaim {
    let at = OffsetDateTime::UNIX_EPOCH;
    AlertDeliveryClaim {
        delivery_id: AlertDeliveryId::from_uuid(Uuid::from_u128(1)),
        alert_id: AlertId::from_uuid(Uuid::from_u128(2)),
        claim_id: AlertClaimId::from_uuid(Uuid::from_u128(3)),
        attempt: 4,
        claimed_at: at,
        first_attempt_at: at,
        lease_until: at + time::Duration::seconds(30),
        message: AlertEmailMessage {
            idempotency_key: "alert-1".into(),
            from_email: "qadra@example.test".into(),
            recipient_email: "operator@example.test".into(),
            login_url: "https://qadra.example.test/login".into(),
            template: AlertEmailTemplate::GenericLoginV1,
        },
    }
}
