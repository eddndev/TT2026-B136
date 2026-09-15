#[allow(dead_code)]
mod case_stage_support;
#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;

use application::case_stages::*;
use application::ApplicationError;
use case_stage_support::*;
use domain::cases::CaseId;
use domain::identity::Role;
use std::sync::atomic::Ordering;

#[test]
fn adoption_returns_the_committed_projection_without_another_read() {
    for role in [Role::Owner, Role::Litigator] {
        let (identity, actor) = identity(role, 2);
        let case_id = CaseId::new();
        let record = crypto::processor()
            .prepare("support.pdf", b"support")
            .unwrap();
        let change = adoption(&record, CaseStage::Trial);
        let expected_result =
            committed(case_id, actor.id, 1, CaseStageChange::Adopt(change.clone()));
        let result = expected_result.clone();
        let checked = record.clone();
        let validator = validator(std::slice::from_ref(&record));
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .withf(move |id, case, expected, _, limits| {
                *id == actor.id
                    && *case == case_id
                    && *expected == CaseStageExpectation::Unregistered
                    && limits.vault().max_vault_bytes() == 16_777_313
            })
            .times(1)
            .return_once(move |_, _, _, _, _| {
                Ok(CaseStagePreparation {
                    current: CurrentCaseStage::Unregistered,
                    records: vec![record],
                })
            });
        store
            .expect_commit()
            .withf(move |id, case, expected, prepared, at| {
                assert_eq!(prepared.supports().len(), 1);
                assert_eq!(prepared.supports()[0].record(), &checked);
                assert_eq!(prepared.supports()[0].format(), StageDocumentFormat::Pdf);
                assert_eq!(
                    prepared.supports()[0].policy(),
                    StageFormatPolicy::PdfDocxV1
                );
                assert_eq!(prepared.supports()[0].snapshot().name, "support.pdf");
                *id == actor.id
                    && *case == case_id
                    && *expected == CaseStageExpectation::Unregistered
                    && *at == instant()
            })
            .times(1)
            .return_once(move |_, _, _, _, _| Ok(result));
        let (service, clock) = service(store, identity, validator.clone());
        assert_eq!(
            service
                .adopt(
                    "session",
                    case_id,
                    CaseStageExpectation::Unregistered,
                    change
                )
                .unwrap(),
            expected_result
        );
        assert_eq!(clock.calls(), 1);
        assert_eq!(validator.calls.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn reads_pass_actor_and_captured_time_without_mutation_permissions() {
    for role in [Role::Owner, Role::Litigator, Role::Paralegal] {
        let (identity, actor) = identity(role, 2);
        let case_id = CaseId::new();
        let mut store = MockStore::new();
        store
            .expect_get()
            .withf(move |id, case, at| *id == actor.id && *case == case_id && *at == instant())
            .times(1)
            .returning(move |_, _, _| {
                Ok(CaseStageDetail {
                    case_id,
                    current: CurrentCaseStage::Unregistered,
                })
            });
        store
            .expect_history()
            .withf(move |id, case, query, at| {
                *id == actor.id
                    && *case == case_id
                    && query.limit() == 1
                    && query.before_revision() == Some(CaseStageRevision::new(2).unwrap())
                    && *at == instant()
            })
            .times(1)
            .returning(|_, _, _, _| {
                Ok(CaseStagePage {
                    entries: vec![],
                    has_more: false,
                    next_before_revision: None,
                })
            });
        let (service, clock) = service(store, identity, validator(&[]));
        assert_eq!(
            service.get("session", case_id).unwrap().current,
            CurrentCaseStage::Unregistered
        );
        assert!(
            !service
                .history("session", case_id, CaseStageQuery::new(1, Some(2)).unwrap())
                .unwrap()
                .has_more
        );
        assert_eq!(clock.calls(), 2);
    }
}

#[test]
fn denied_roles_never_prepare_or_read_storage() {
    let record = crypto::processor()
        .prepare("support.pdf", b"support")
        .unwrap();
    for role in [Role::Paralegal, Role::Client] {
        let (identity, _) = identity(role, 2);
        let (service, clock) = service(MockStore::new(), identity, validator(&[]));
        assert!(matches!(
            service.adopt(
                "session",
                CaseId::new(),
                CaseStageExpectation::Unregistered,
                adoption(&record, CaseStage::Investigation)
            ),
            Err(ApplicationError::PermissionDenied)
        ));
        assert!(matches!(
            service.transition(
                "session",
                CaseId::new(),
                CaseStageRevision::FIRST,
                StageTransition::to_intermediate(
                    DeclaredStageTime::instant(instant()).unwrap(),
                    reference(&record),
                    None
                )
            ),
            Err(ApplicationError::PermissionDenied)
        ));
        assert_eq!(clock.calls(), 0);
    }
    let (identity, _) = identity(Role::Client, 2);
    let (service, clock) = service(MockStore::new(), identity, validator(&[]));
    assert!(matches!(
        service.get("session", CaseId::new()),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(matches!(
        service.history(
            "session",
            CaseId::new(),
            CaseStageQuery::new(20, None).unwrap()
        ),
        Err(ApplicationError::PermissionDenied)
    ));
    assert_eq!(clock.calls(), 0);
}

#[test]
fn ordinary_transition_preserves_initial_entry_and_separate_revision() {
    let (identity, actor) = identity(Role::Litigator, 2);
    let case_id = CaseId::new();
    let record = crypto::processor()
        .prepare("accusation.pdf", b"accusation")
        .unwrap();
    let transition = StageTransition::to_intermediate(
        DeclaredStageTime::instant(instant()).unwrap(),
        reference(&record),
        None,
    );
    let expected_result = committed(
        case_id,
        actor.id,
        2,
        CaseStageChange::Transition(transition.clone()),
    );
    let result = expected_result.clone();
    let mut store = MockStore::new();
    let validator = validator(std::slice::from_ref(&record));
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _, _| {
            Ok(CaseStagePreparation {
                current: initial(case_id, actor.id),
                records: vec![record],
            })
        });
    store
        .expect_commit()
        .withf(move |_, _, expected, prepared, _| {
            *expected == CaseStageExpectation::Revision(CaseStageRevision::FIRST)
                && prepared.change().stage() == CaseStage::Intermediate
        })
        .times(1)
        .return_once(move |_, _, _, _, _| Ok(result));
    let (service, _) = service(store, identity, validator);
    assert_eq!(
        service
            .transition("session", case_id, CaseStageRevision::FIRST, transition)
            .unwrap(),
        expected_result
    );
}

#[test]
fn trial_supports_are_reordered_and_deduplicated_in_one_parser_batch() {
    for duplicate in [false, true] {
        let (identity, actor) = identity(Role::Owner, 2);
        let case_id = CaseId::new();
        let processor = crypto::processor();
        let order = processor.prepare("order.pdf", b"order").unwrap();
        let receipt = if duplicate {
            order.clone()
        } else {
            processor.prepare("receipt.docx", b"receipt").unwrap()
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
            7,
            CaseStageChange::Adopt(adoption(&order, CaseStage::Intermediate)),
        )
        .current;
        let record_count = if duplicate { 1 } else { 2 };
        let records = if duplicate {
            vec![order.clone()]
        } else {
            vec![receipt.clone(), order.clone()]
        };
        let validator = validator(&if duplicate {
            vec![order.clone()]
        } else {
            vec![order, receipt]
        });
        let result = committed(
            case_id,
            actor.id,
            8,
            CaseStageChange::Transition(transition.clone()),
        );
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _, _| Ok(CaseStagePreparation { current, records }));
        store
            .expect_commit()
            .withf(move |_, _, _, prepared, _| prepared.supports().len() == record_count)
            .times(1)
            .return_once(move |_, _, _, _, _| Ok(result));
        let (service, _) = service(store, identity, validator.clone());
        assert_eq!(
            service
                .transition(
                    "session",
                    case_id,
                    CaseStageRevision::new(7).unwrap(),
                    transition
                )
                .unwrap()
                .current
                .stage(),
            Some(CaseStage::Trial)
        );
        assert_eq!(validator.calls.load(Ordering::SeqCst), 1);
    }
}
