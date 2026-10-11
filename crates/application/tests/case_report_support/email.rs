use application::{case_reports::*, ApplicationError};
use domain::clock::{Clock, OffsetDateTime};
use std::sync::{Arc, Mutex};
use time::Duration;
use uuid::Uuid;

mockall::mock! { Store {} impl CaseReportEmailStore for Store {
    fn claim_delivery(&self, claim:CaseReportEmailClaim, template:&CaseReportEmailTemplate)->Result<Option<CaseReportEmailLease>,ApplicationError>;
    fn authorize_delivery(&self, lease:&CaseReportEmailLease, at:OffsetDateTime)->Result<Option<CaseReportEmailMessage>,ApplicationError>;
    fn finish_delivery(&self, result:CaseReportEmailFinish)->Result<bool,ApplicationError>;
} }
mockall::mock! { Delivery {} impl CaseReportEmailDelivery for Delivery {
    fn deliver(&self, id:Uuid, message:&CaseReportEmailMessage)->Result<CaseReportEmailOutcome,ApplicationError>;
} }
mockall::mock! { Time {} impl Clock for Time {
    fn now(&self)->OffsetDateTime;
} }

pub(super) struct State {
    pub(super) now: OffsetDateTime,
    pub(super) available: Option<CaseReportEmailLease>,
    pub(super) claimed: Option<CaseReportEmailLease>,
    pub(super) authorized: bool,
    pub(super) changed_message: Option<CaseReportEmailMessage>,
    pub(super) authorize_delay: Duration,
    pub(super) wrong_lease: bool,
    pub(super) fail_at: Option<&'static str>,
    pub(super) outcome: CaseReportEmailOutcome,
    pub(super) accepts_finish: bool,
    pub(super) events: Vec<&'static str>,
    pub(super) templates: Vec<CaseReportEmailTemplate>,
    pub(super) delivered: Vec<(Uuid, CaseReportEmailMessage)>,
    pub(super) finishes: Vec<CaseReportEmailFinish>,
}

pub(super) fn template() -> CaseReportEmailTemplate {
    CaseReportEmailTemplate {
        from: "reports@example.test".into(),
        subject: "Qadra: informe disponible".into(),
        text: "Tu informe esta listo. Entra en https://qadra.example.test/#reports".into(),
    }
}

pub(super) fn harness() -> (CaseReportEmailWorker, Arc<Mutex<State>>) {
    let now = OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap();
    let template = template();
    let state = Arc::new(Mutex::new(State {
        now,
        available: Some(CaseReportEmailLease {
            report_id: CaseReportId::from_uuid(Uuid::from_u128(1)),
            delivery_id: Uuid::from_u128(2),
            generation: 1,
            lease_id: Uuid::nil(),
            lease_until: now + Duration::seconds(30),
            provider_deadline: now + Duration::hours(24),
            message: CaseReportEmailMessage {
                from: template.from.clone(),
                to: "requester@example.test".into(),
                subject: template.subject.clone(),
                text: template.text.clone(),
            },
        }),
        claimed: None,
        authorized: true,
        changed_message: None,
        authorize_delay: Duration::ZERO,
        wrong_lease: false,
        fail_at: None,
        outcome: CaseReportEmailOutcome::Accepted,
        accepts_finish: true,
        events: Vec::new(),
        templates: Vec::new(),
        delivered: Vec::new(),
        finishes: Vec::new(),
    }));
    (worker(state.clone(), template), state)
}

fn failure() -> ApplicationError {
    ApplicationError::Port("private-provider-or-database-detail".into())
}

pub(super) fn worker(
    state: Arc<Mutex<State>>,
    template: CaseReportEmailTemplate,
) -> CaseReportEmailWorker {
    let mut store = MockStore::new();
    let shared = state.clone();
    store
        .expect_claim_delivery()
        .returning(move |claim, template| {
            let mut state = shared.lock().unwrap();
            state.events.push("claim");
            state.templates.push(template.clone());
            assert_eq!(claim.at, state.now);
            assert!(!claim.lease_id.is_nil());
            assert_eq!(
                (claim.lease_seconds, claim.provider_window_seconds),
                (30, 86_400)
            );
            if state.fail_at == Some("claim") {
                return Err(failure());
            }
            let mut lease = state.available.clone();
            if let Some(value) = &mut lease {
                if !state.wrong_lease {
                    value.lease_id = claim.lease_id;
                }
            }
            state.claimed = lease.clone();
            Ok(lease)
        });
    let shared = state.clone();
    store
        .expect_authorize_delivery()
        .returning(move |lease, at| {
            let mut state = shared.lock().unwrap();
            state.events.push("authorize");
            assert_eq!(Some(lease), state.claimed.as_ref());
            assert_eq!(at, state.now);
            let delay = state.authorize_delay;
            state.now += delay;
            if state.fail_at == Some("authorize") {
                return Err(failure());
            }
            Ok(state.authorized.then(|| {
                state
                    .changed_message
                    .clone()
                    .unwrap_or_else(|| lease.message.clone())
            }))
        });
    let shared = state.clone();
    store.expect_finish_delivery().returning(move |finish| {
        let mut state = shared.lock().unwrap();
        state.events.push("finish");
        let lease = state.claimed.as_ref().unwrap();
        assert_eq!(
            (
                finish.report_id,
                finish.delivery_id,
                finish.generation,
                finish.lease_id
            ),
            (
                lease.report_id,
                lease.delivery_id,
                lease.generation,
                lease.lease_id
            )
        );
        assert_eq!(finish.at, state.now);
        state.finishes.push(finish);
        if state.fail_at == Some("finish") {
            return Err(failure());
        }
        Ok(state.accepts_finish)
    });
    let mut delivery = MockDelivery::new();
    let shared = state.clone();
    delivery.expect_deliver().returning(move |id, message| {
        let mut state = shared.lock().unwrap();
        assert_eq!(state.events.last(), Some(&"authorize"));
        state.events.push("deliver");
        state.delivered.push((id, message.clone()));
        if state.fail_at == Some("deliver") {
            return Err(failure());
        }
        Ok(state.outcome)
    });
    let mut clock = MockTime::new();
    clock
        .expect_now()
        .returning(move || state.lock().unwrap().now);
    CaseReportEmailWorker::new(
        Arc::new(store),
        Arc::new(delivery),
        template,
        Arc::new(clock),
    )
}
