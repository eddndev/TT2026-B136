#[allow(dead_code)]
mod case_stage_support;
#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[path = "document_format_support/mod.rs"]
mod format_support;

use application::case_stages::*;
use application::documents::{DocumentFormatBatch, DocumentFormatBatchValidator};
use application::ApplicationError;
use case_stage_support::*;
use domain::cases::CaseId;
use domain::identity::Role;
use mockall::Sequence;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

struct OrderedValidator {
    clock: Arc<CountingClock>,
    calls: AtomicUsize,
    count: usize,
}
impl DocumentFormatBatchValidator for OrderedValidator {
    fn validate_batch(
        &self,
        batch: &DocumentFormatBatch<'_>,
    ) -> Result<Vec<StageDocumentFormat>, ApplicationError> {
        assert_eq!(self.clock.calls(), 0);
        assert_eq!(batch.inputs().len(), self.count);
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(vec![StageDocumentFormat::Pdf; self.count])
    }
}

#[test]
fn crypto_runs_once_per_exact_support_before_reauthentication_and_clock_capture() {
    for duplicate in [false, true] {
        let (_, actor) = identity(Role::Owner, 0);
        let case_id = CaseId::new();
        let producer = crypto::processor();
        let order = producer
            .seal(&producer.prepare("order.pdf", b"order").unwrap())
            .unwrap();
        let receipt = if duplicate {
            order.clone()
        } else {
            producer.prepare("receipt.pdf", b"receipt").unwrap()
        };
        let transition = StageTransition::to_trial(
            DeclaredStageTime::instant(instant()).unwrap(),
            reference(&order),
            DeclaredStageTime::instant(instant()).unwrap(),
            StageCourt::new("Court").unwrap(),
            None,
            Some(reference(&receipt)),
            None,
        )
        .unwrap();
        let current = committed(
            case_id,
            actor.id,
            1,
            CaseStageChange::Adopt(adoption(&order, CaseStage::Intermediate)),
        )
        .current;
        let result = committed(
            case_id,
            actor.id,
            2,
            CaseStageChange::Transition(transition.clone()),
        );
        let records = if duplicate {
            vec![order.clone()]
        } else {
            vec![order.clone(), receipt]
        };
        let count = records.len();
        let clock = Arc::new(CountingClock::default());
        let validator = Arc::new(OrderedValidator {
            clock: clock.clone(),
            calls: AtomicUsize::new(0),
            count,
        });
        let mut identity = MockIdentity::new();
        let mut sequence = Sequence::new();
        let first = actor.clone();
        identity
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| Ok(first));
        let checked_clock = clock.clone();
        let checked_validator = validator.clone();
        identity
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| {
                assert_eq!(checked_clock.calls(), 0);
                assert_eq!(checked_validator.calls.load(Ordering::SeqCst), 1);
                Ok(actor)
            });
        let mut store = MockStore::new();
        let checked_clock = clock.clone();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _, _| {
                assert_eq!(checked_clock.calls(), 0);
                Ok(CaseStagePreparation { current, records })
            });
        let checked_clock = clock.clone();
        store
            .expect_commit()
            .times(1)
            .return_once(move |_, _, _, prepared, at| {
                assert_eq!(checked_clock.calls(), 1);
                assert_eq!(at, instant());
                assert_eq!(prepared.supports().len(), count);
                assert_eq!(prepared.supports()[0].record(), &order);
                Ok(result)
            });
        let (processor, observations) = format_support::processor();
        let service = CaseStageService::new(
            Arc::new(store),
            Arc::new(identity),
            Arc::new(processor),
            validator,
            clock,
        );
        service
            .transition("session", case_id, CaseStageRevision::FIRST, transition)
            .unwrap();
        let events = &observations.lock().unwrap().events;
        for event in ["unwrap", "open", "hash"] {
            assert_eq!(
                events.iter().filter(|value| **value == event).count(),
                count
            );
        }
        for event in ["signature", "timestamp", "certificate"] {
            assert_eq!(events.iter().filter(|value| **value == event).count(), 1);
        }
    }
}

#[test]
fn prepare_denials_return_before_crypto_parser_and_second_authentication() {
    for error in [
        ApplicationError::CaseNotFound,
        ApplicationError::DocumentNotFound("not found".into()),
        ApplicationError::CaseClosed,
        ApplicationError::CaseStageProfileIncomplete,
        ApplicationError::PermissionDenied,
        ApplicationError::CaseStageConflict,
    ] {
        let expected = error.to_string();
        let (identity, _) = identity(Role::Owner, 1);
        let record = crypto::processor()
            .prepare("support.pdf", b"support")
            .unwrap();
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _, _| Err(error));
        let check = validator(&[]);
        let (service, clock) = service(store, identity, check.clone());
        assert_eq!(
            service
                .adopt(
                    "session",
                    CaseId::new(),
                    CaseStageExpectation::Unregistered,
                    adoption(&record, CaseStage::Investigation)
                )
                .unwrap_err()
                .to_string(),
            expected
        );
        assert_eq!(check.calls.load(Ordering::SeqCst), 0);
        assert_eq!(clock.calls(), 0);
    }
}

#[test]
fn an_invalid_session_prevents_all_storage_and_clock_access() {
    let mut identity = MockIdentity::new();
    identity
        .expect_authenticate()
        .times(4)
        .returning(|_| Err(ApplicationError::InvalidSession));
    let record = crypto::processor()
        .prepare("support.pdf", b"support")
        .unwrap();
    let (service, clock) = service(MockStore::new(), identity, validator(&[]));
    let case_id = CaseId::new();
    assert!(matches!(
        service.get("session", case_id),
        Err(ApplicationError::InvalidSession)
    ));
    assert!(matches!(
        service.history("session", case_id, CaseStageQuery::new(20, None).unwrap()),
        Err(ApplicationError::InvalidSession)
    ));
    assert!(matches!(
        service.adopt(
            "session",
            case_id,
            CaseStageExpectation::Unregistered,
            adoption(&record, CaseStage::Investigation)
        ),
        Err(ApplicationError::InvalidSession)
    ));
    assert!(matches!(
        service.transition(
            "session",
            case_id,
            CaseStageRevision::FIRST,
            StageTransition::to_intermediate(
                DeclaredStageTime::instant(instant()).unwrap(),
                reference(&record),
                None
            )
        ),
        Err(ApplicationError::InvalidSession)
    ));
    assert_eq!(clock.calls(), 0);
}

#[test]
fn failed_read_audit_returns_no_current_or_history_projection() {
    let (identity, _) = identity(Role::Paralegal, 2);
    let mut store = MockStore::new();
    store
        .expect_get()
        .times(1)
        .returning(|_, _, _| Err(ApplicationError::Port("audit unavailable".into())));
    store
        .expect_history()
        .times(1)
        .returning(|_, _, _, _| Err(ApplicationError::Port("audit unavailable".into())));
    let (service, clock) = service(store, identity, validator(&[]));
    assert!(matches!(
        service.get("session", CaseId::new()),
        Err(ApplicationError::Port(_))
    ));
    assert!(matches!(
        service.history(
            "session",
            CaseId::new(),
            CaseStageQuery::new(20, None).unwrap()
        ),
        Err(ApplicationError::Port(_))
    ));
    assert_eq!(clock.calls(), 2);
}
