#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
mod hearing_result_support;
#[allow(dead_code)]
mod hearing_support;

use application::{hearing_results::*, ApplicationError};
use domain::{
    cases::CaseId,
    identity::{Role, UserId},
};
use hearing_result_support::*;
use std::sync::Arc;
#[test]
fn withdrawn_exact_antecedent_can_continue_as_a_new_root_without_changing_its_state() {
    let (identity, actor) = identity(Role::Owner, 2);
    let case_id = CaseId::new();
    let mut prior = preparation(case_id, actor.id);
    let base = detail(actor.id, &command(&prior), &prior);
    let withdrawn_command = withdrawal(&base);
    prior.base = Some(base);
    let old = detail(actor.id, &withdrawn_command, &prior);
    let mut prep = preparation(case_id, actor.id);
    let mut cmd = command(&prep);
    if let HearingResultChange::Record { continuation, .. } = &mut cmd.change {
        *continuation = Some(HearingResultContinuationRef::new(
            old.snapshot.id,
            old.snapshot.revision,
        ));
    }
    let expected = previous(&old.snapshot);
    prep.continuation = Some(old.snapshot);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(prep));
    let (service, _) = service(store, identity, Arc::new(Validator::default()));
    let draft = service.prepare("session", case_id, cmd).unwrap();
    assert_eq!(draft.continuation, Some(expected));
    assert_ne!(draft.command.result_id, expected.reference.result_id);
    assert_eq!(draft.result_revision.get(), 1);
}
#[test]
fn changed_authenticated_actor_prevents_commit_after_work() {
    let (_, actor) = identity(Role::Owner, 0);
    let case_id = CaseId::new();
    let prep = preparation(case_id, actor.id);
    let cmd = command(&prep);
    let digest = detail(actor.id, &cmd, &prep)
        .snapshot
        .receipt
        .submission_digest;
    let mut auth = MockIdentity::new();
    let mut order = mockall::Sequence::new();
    let first = actor.clone();
    auth.expect_authenticate()
        .times(1)
        .in_sequence(&mut order)
        .return_once(move |_| Ok(first));
    auth.expect_authenticate()
        .times(1)
        .in_sequence(&mut order)
        .return_once(move |_| {
            Ok(application::identity::Principal {
                id: UserId::new(),
                ..actor
            })
        });
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(prep));
    let (service, _) = service(store, auth, Arc::new(Validator::default()));
    assert!(matches!(
        service.submit("session", case_id, cmd, digest),
        Err(ApplicationError::InvalidSession)
    ));
}
#[test]
fn another_valid_committed_receipt_cannot_confirm_the_requested_operation() {
    let (identity, actor) = identity(Role::Owner, 2);
    let case_id = CaseId::new();
    let prep = preparation(case_id, actor.id);
    let cmd = command(&prep);
    let digest = detail(actor.id, &cmd, &prep)
        .snapshot
        .receipt
        .submission_digest;
    let mut other = cmd.clone();
    other.operation_id = HearingResultOperationId::new();
    let returned = detail(actor.id, &other, &prep);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(prep));
    store
        .expect_commit()
        .times(1)
        .return_once(move |_, _, _| Ok(returned));
    let (service, _) = service(store, identity, Arc::new(Validator::default()));
    assert!(matches!(
        service.submit("session", case_id, cmd, digest),
        Err(ApplicationError::HearingResult(
            HearingResultError::StoredInconsistent(_)
        ))
    ));
}
#[test]
fn commit_failures_propagate_without_automatic_retries() {
    for error in [
        ApplicationError::HearingResult(HearingResultError::OperationConflict),
        ApplicationError::HearingResult(HearingResultError::RevisionConflict),
        ApplicationError::StageSupportChanged,
        ApplicationError::Port("audit unavailable".into()),
    ] {
        let expected = match error {
            ApplicationError::StageSupportChanged => HearingResultError::SupportChanged.to_string(),
            _ => error.to_string(),
        };
        let (identity, actor) = identity(Role::Owner, 2);
        let case_id = CaseId::new();
        let prep = preparation(case_id, actor.id);
        let cmd = command(&prep);
        let digest = detail(actor.id, &cmd, &prep)
            .snapshot
            .receipt
            .submission_digest;
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _| Ok(prep));
        store
            .expect_commit()
            .times(1)
            .return_once(move |_, _, _| Err(error));
        let (service, _) = service(store, identity, Arc::new(Validator::default()));
        assert_eq!(
            service
                .submit("session", case_id, cmd, digest)
                .unwrap_err()
                .to_string(),
            expected
        );
    }
}
