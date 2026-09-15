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
use domain::crypto::Sha256Digest;
use domain::identity::{Role, UserId};
use domain::DomainError;
use mockall::Sequence;
use std::sync::{atomic::Ordering, Arc};

#[test]
fn adoption_with_positive_expectation_is_rejected_before_preparation() {
    let (identity, _) = identity(Role::Owner, 1);
    let record = crypto::processor()
        .prepare("support.pdf", b"support")
        .unwrap();
    let (service, clock) = service(MockStore::new(), identity, validator(&[]));
    assert!(matches!(
        service.adopt(
            "session",
            CaseId::new(),
            CaseStageExpectation::Revision(CaseStageRevision::FIRST),
            adoption(&record, CaseStage::Investigation)
        ),
        Err(ApplicationError::InvalidInput(_))
    ));
    assert_eq!(clock.calls(), 0);
}

#[test]
fn conflicting_missing_wrong_edge_or_exhausted_heads_do_not_reach_parser() {
    for scenario in 0..6 {
        let (identity, actor) = identity(Role::Owner, 1);
        let case_id = CaseId::new();
        let record = crypto::processor()
            .prepare("support.pdf", b"support")
            .unwrap();
        let transition = StageTransition::to_intermediate(
            DeclaredStageTime::instant(instant()).unwrap(),
            reference(&record),
            None,
        );
        let current = match scenario {
            0 => CurrentCaseStage::Unregistered,
            1 => {
                committed(
                    case_id,
                    actor.id,
                    7,
                    CaseStageChange::Adopt(adoption(&record, CaseStage::Investigation)),
                )
                .current
            }
            2 => {
                committed(
                    case_id,
                    actor.id,
                    1,
                    CaseStageChange::Adopt(adoption(&record, CaseStage::Trial)),
                )
                .current
            }
            3 => {
                committed(
                    case_id,
                    actor.id,
                    u32::MAX,
                    CaseStageChange::Adopt(adoption(&record, CaseStage::Investigation)),
                )
                .current
            }
            4 => initial(CaseId::new(), actor.id),
            _ => initial(case_id, actor.id),
        };
        let adopt = adoption(&record, CaseStage::Investigation);
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _, _| {
                Ok(CaseStagePreparation {
                    current,
                    records: vec![record],
                })
            });
        let check = validator(&[]);
        let (service, clock) = service(store, identity, check.clone());
        let result = if scenario == 5 {
            service.adopt(
                "session",
                case_id,
                CaseStageExpectation::Unregistered,
                adopt,
            )
        } else {
            service.transition(
                "session",
                case_id,
                CaseStageRevision::new(if scenario == 3 { u32::MAX } else { 1 }).unwrap(),
                transition,
            )
        };
        assert!(matches!(
            (scenario, result),
            (0, Err(ApplicationError::CaseStageRequired))
                | (1 | 5, Err(ApplicationError::CaseStageConflict))
                | (2, Err(ApplicationError::CaseStageTransitionRejected))
                | (3, Err(ApplicationError::CaseStageRevisionExhausted))
                | (4, Err(ApplicationError::StoredCaseStageInconsistent(_)))
        ));
        assert_eq!(check.calls.load(Ordering::SeqCst), 0);
        assert_eq!(clock.calls(), 0);
    }
}

#[test]
fn missing_extra_foreign_or_duplicate_prepared_records_are_port_inconsistencies() {
    for scenario in 0..4 {
        let (identity, _) = identity(Role::Owner, 1);
        let processor = crypto::processor();
        let record = processor.prepare("support.pdf", b"support").unwrap();
        let change = adoption(&record, CaseStage::Investigation);
        let other = processor.prepare("other.pdf", b"other").unwrap();
        let records = match scenario {
            0 => vec![],
            1 => vec![record, other],
            2 => vec![other],
            _ => vec![record.clone(), record],
        };
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _, _| {
                Ok(CaseStagePreparation {
                    current: CurrentCaseStage::Unregistered,
                    records,
                })
            });
        let check = validator(&[]);
        let (service, clock) = service(store, identity, check.clone());
        assert!(matches!(
            service.adopt(
                "session",
                CaseId::new(),
                CaseStageExpectation::Unregistered,
                change
            ),
            Err(ApplicationError::StoredCaseStageInconsistent(_))
        ));
        assert_eq!(check.calls.load(Ordering::SeqCst), 0);
        assert_eq!(clock.calls(), 0);
    }
}

#[test]
fn mismatched_selected_digest_does_not_silently_refresh_the_support() {
    let (identity, _) = identity(Role::Owner, 1);
    let mut record = crypto::processor()
        .prepare("support.pdf", b"support")
        .unwrap();
    let change = adoption(&record, CaseStage::Investigation);
    record.digest = Sha256Digest::from_array([99; 32]);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _, _| {
            Ok(CaseStagePreparation {
                current: CurrentCaseStage::Unregistered,
                records: vec![record],
            })
        });
    let (service, clock) = service(store, identity, validator(&[]));
    assert!(matches!(
        service.adopt(
            "session",
            CaseId::new(),
            CaseStageExpectation::Unregistered,
            change
        ),
        Err(ApplicationError::StageSupportDigestMismatch)
    ));
    assert_eq!(clock.calls(), 0);
}

#[test]
fn parser_or_cryptographic_failure_cannot_commit_a_stage() {
    for corrupt in [false, true] {
        let (identity, _) = identity(Role::Owner, 1);
        let mut record = crypto::processor()
            .prepare("support.pdf", b"support")
            .unwrap();
        let change = adoption(&record, CaseStage::Investigation);
        let mut check = validator(std::slice::from_ref(&record));
        Arc::get_mut(&mut check).unwrap().failure = !corrupt;
        if corrupt {
            record.vault[81] ^= 1;
        }
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _, _| {
                Ok(CaseStagePreparation {
                    current: CurrentCaseStage::Unregistered,
                    records: vec![record],
                })
            });
        let (service, clock) = service(store, identity, check.clone());
        let result = service.adopt(
            "session",
            CaseId::new(),
            CaseStageExpectation::Unregistered,
            change,
        );
        assert!(matches!(
            (corrupt, result),
            (false, Err(ApplicationError::StageSupportFormatRejected))
                | (true, Err(ApplicationError::StoredDocumentInconsistent(_)))
        ));
        assert_eq!(check.calls.load(Ordering::SeqCst), usize::from(!corrupt));
        assert_eq!(clock.calls(), 0);
    }
}

#[test]
fn session_revocation_role_loss_and_actor_replacement_after_parser_prevent_commit() {
    for scenario in 0..3 {
        let (_, actor) = identity(Role::Owner, 0);
        let record = crypto::processor()
            .prepare("support.pdf", b"support")
            .unwrap();
        let change = adoption(&record, CaseStage::Investigation);
        let check = validator(std::slice::from_ref(&record));
        let mut sequence = Sequence::new();
        let mut identity = MockIdentity::new();
        let before = actor.clone();
        identity
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| Ok(before));
        let mut after = actor;
        if scenario == 1 {
            after.role = Role::Paralegal;
        }
        if scenario == 2 {
            after.id = UserId::new();
        }
        let observed = check.clone();
        identity
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| {
                assert_eq!(observed.calls.load(Ordering::SeqCst), 1);
                if scenario == 0 {
                    Err(ApplicationError::InvalidSession)
                } else {
                    Ok(after)
                }
            });
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _, _| {
                Ok(CaseStagePreparation {
                    current: CurrentCaseStage::Unregistered,
                    records: vec![record],
                })
            });
        let (service, clock) = service(store, identity, check);
        let result = service.adopt(
            "session",
            CaseId::new(),
            CaseStageExpectation::Unregistered,
            change,
        );
        assert!(matches!(
            (scenario, result),
            (0 | 2, Err(ApplicationError::InvalidSession))
                | (1, Err(ApplicationError::PermissionDenied))
        ));
        assert_eq!(clock.calls(), 0);
    }
}

#[test]
fn commit_rejections_are_returned_without_retry_or_successful_post_read() {
    for rejection in [
        ApplicationError::CaseClosed,
        ApplicationError::CaseStageProfileIncomplete,
        ApplicationError::CaseStageConflict,
        ApplicationError::StageSupportChanged,
        ApplicationError::PermissionDenied,
        ApplicationError::CaseNotFound,
    ] {
        let expected = rejection.to_string();
        let (identity, _) = identity(Role::Owner, 2);
        let record = crypto::processor()
            .prepare("support.pdf", b"support")
            .unwrap();
        let change = adoption(&record, CaseStage::Investigation);
        let check = validator(std::slice::from_ref(&record));
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _, _| {
                Ok(CaseStagePreparation {
                    current: CurrentCaseStage::Unregistered,
                    records: vec![record],
                })
            });
        store
            .expect_commit()
            .times(1)
            .return_once(move |_, _, _, _, _| Err(rejection));
        let (service, clock) = service(store, identity, check);
        assert_eq!(
            service
                .adopt(
                    "session",
                    CaseId::new(),
                    CaseStageExpectation::Unregistered,
                    change
                )
                .unwrap_err()
                .to_string(),
            expected
        );
        assert_eq!(clock.calls(), 1);
    }
}

#[test]
fn captured_clock_after_preparation_rejects_future_declarations_without_commit() {
    let (identity, _) = identity(Role::Owner, 2);
    let record = crypto::processor()
        .prepare("support.pdf", b"support")
        .unwrap();
    let change = StageAdoption::new(
        CaseStage::Investigation,
        DeclaredStageTime::instant(instant() + time::Duration::NANOSECOND).unwrap(),
        StageNote::new("Declared history").unwrap(),
        reference(&record),
    );
    let check = validator(std::slice::from_ref(&record));
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _, _| {
            Ok(CaseStagePreparation {
                current: CurrentCaseStage::Unregistered,
                records: vec![record],
            })
        });
    let (service, clock) = service(store, identity, check.clone());
    assert!(matches!(
        service.adopt(
            "session",
            CaseId::new(),
            CaseStageExpectation::Unregistered,
            change
        ),
        Err(ApplicationError::Domain(DomainError::StageActInFuture))
    ));
    assert_eq!(check.calls.load(Ordering::SeqCst), 1);
    assert_eq!(clock.calls(), 1);
}
