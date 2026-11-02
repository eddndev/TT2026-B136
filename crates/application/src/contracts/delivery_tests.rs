use super::*;
use std::sync::Mutex;

const NOW: i64 = 10_000;

fn at(seconds: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(seconds).unwrap()
}

struct FixedClock;
impl Clock for FixedClock {
    fn now(&self) -> OffsetDateTime {
        at(NOW)
    }
}

#[derive(Default)]
struct Store {
    lease: Mutex<Option<ContractNoticeLease>>,
    claims: Mutex<Vec<ContractNoticeClaim>>,
    finished: Mutex<Vec<ContractNoticeFinish>>,
    stale_finish: bool,
    claim_fails: bool,
}

impl ContractNoticeStore for Store {
    fn claim_notice(
        &self,
        claim: ContractNoticeClaim,
    ) -> Result<Option<ContractNoticeLease>, ApplicationError> {
        if self.claim_fails {
            return Err(ApplicationError::Port("private storage detail".into()));
        }
        self.claims.lock().unwrap().push(claim.clone());
        Ok(self.lease.lock().unwrap().take().map(|mut lease| {
            lease.lease_id = claim.lease_id;
            lease
        }))
    }
    fn finish_notice(&self, finish: ContractNoticeFinish) -> Result<bool, ApplicationError> {
        self.finished.lock().unwrap().push(finish);
        Ok(!self.stale_finish)
    }
}

struct Provider(
    Result<CaseReportEmailOutcome, ApplicationError>,
    Mutex<Vec<Uuid>>,
);
impl CaseReportEmailDelivery for Provider {
    fn deliver(
        &self,
        delivery_id: Uuid,
        message: &CaseReportEmailMessage,
    ) -> Result<CaseReportEmailOutcome, ApplicationError> {
        assert_eq!(message.to, "client@example.test");
        self.1.lock().unwrap().push(delivery_id);
        match &self.0 {
            Ok(outcome) => Ok(*outcome),
            Err(_) => Err(ApplicationError::Port("timeout".into())),
        }
    }
}

fn lease(until: i64, deadline: i64) -> ContractNoticeLease {
    ContractNoticeLease {
        id: Uuid::from_u128(7),
        generation: 3,
        lease_id: Uuid::nil(),
        lease_until: at(until),
        provider_deadline: at(deadline),
        message: CaseReportEmailMessage {
            from: "qadra@example.test".into(),
            to: "client@example.test".into(),
            subject: "Qadra: Solicitud de firma de contrato".into(),
            text: "Tienes una solicitud de firma de contrato pendiente.".into(),
        },
    }
}

fn run(
    store: Store,
    outcome: Result<CaseReportEmailOutcome, ApplicationError>,
) -> (
    Result<ContractNoticeRun, ApplicationError>,
    Arc<Store>,
    Arc<Provider>,
) {
    let store = Arc::new(store);
    let provider = Arc::new(Provider(outcome, Mutex::default()));
    let worker = ContractNoticeWorker::new(store.clone(), provider.clone(), Arc::new(FixedClock));
    (worker.run_once(), store, provider)
}

fn with_lease(value: ContractNoticeLease) -> Store {
    Store {
        lease: Mutex::new(Some(value)),
        ..Store::default()
    }
}

#[test]
fn an_idle_store_sends_nothing_and_claims_with_the_fixed_lease_and_window() {
    let (result, store, provider) = run(Store::default(), Ok(CaseReportEmailOutcome::Accepted));
    assert_eq!(result.unwrap(), ContractNoticeRun::Idle);
    let claims = store.claims.lock().unwrap();
    assert_eq!(claims[0].lease_seconds, 30);
    assert_eq!(claims[0].provider_window_seconds, 86_400);
    assert!(!claims[0].lease_id.is_nil());
    assert!(provider.1.lock().unwrap().is_empty());
}

#[test]
fn an_accepted_notice_is_sent_once_with_its_own_identity_as_the_provider_key() {
    let (result, store, provider) = run(
        with_lease(lease(NOW + 30, NOW + 86_400)),
        Ok(CaseReportEmailOutcome::Accepted),
    );
    assert_eq!(
        result.unwrap(),
        ContractNoticeRun::Recorded(ContractNoticeState::Sent)
    );
    assert_eq!(*provider.1.lock().unwrap(), vec![Uuid::from_u128(7)]);
    let finished = store.finished.lock().unwrap();
    assert_eq!(finished[0].generation, 3);
    assert_eq!(finished[0].retry_at, None);
}

#[test]
fn an_uncertain_attempt_retries_after_a_minute_only_before_the_deadline() {
    let (result, store, _) = run(
        with_lease(lease(NOW + 30, NOW + 86_400)),
        Err(ApplicationError::Port("timeout".into())),
    );
    assert_eq!(
        result.unwrap(),
        ContractNoticeRun::Recorded(ContractNoticeState::Uncertain)
    );
    assert_eq!(
        store.finished.lock().unwrap()[0].retry_at,
        Some(at(NOW + 60))
    );
    let (_, store, _) = run(
        with_lease(lease(NOW + 30, NOW + 50)),
        Ok(CaseReportEmailOutcome::Uncertain),
    );
    assert_eq!(store.finished.lock().unwrap()[0].retry_at, None);
}

#[test]
fn an_expired_lease_or_a_stale_finish_never_reports_a_recorded_outcome() {
    let (result, store, provider) = run(
        with_lease(lease(NOW, NOW + 86_400)),
        Ok(CaseReportEmailOutcome::Accepted),
    );
    assert_eq!(result.unwrap(), ContractNoticeRun::Stale);
    assert!(provider.1.lock().unwrap().is_empty());
    assert!(store.finished.lock().unwrap().is_empty());
    let mut stale = with_lease(lease(NOW + 30, NOW + 86_400));
    stale.stale_finish = true;
    let (result, _, _) = run(stale, Ok(CaseReportEmailOutcome::DefinitelyRejected));
    assert_eq!(result.unwrap(), ContractNoticeRun::Stale);
}

#[test]
fn a_storage_failure_is_reported_without_its_detail() {
    let store = Store {
        claim_fails: true,
        ..Store::default()
    };
    let (result, _, _) = run(store, Ok(CaseReportEmailOutcome::Accepted));
    match result {
        Err(ApplicationError::Port(message)) => assert!(!message.contains("private")),
        other => panic!("unexpected {other:?}"),
    }
}
