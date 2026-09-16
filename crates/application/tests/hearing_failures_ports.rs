#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
mod hearing_support;
use application::{hearings::*, ApplicationError};
use domain::{
    cases::CaseId,
    crypto::Sha256Digest,
    identity::{Role, UserId},
};
use hearing_support::*;
use std::sync::{atomic::Ordering, Arc};

#[test]
fn invalid_session_denies_before_any_storage_access() {
    let mut identity = MockIdentity::new();
    identity
        .expect_authenticate()
        .times(2)
        .returning(|_| Err(ApplicationError::InvalidSession));
    let (service, clock) = service(MockStore::new(), identity, Arc::new(Validator::default()));
    assert!(matches!(
        service.prepare("session", CaseId::new(), command()),
        Err(ApplicationError::InvalidSession)
    ));
    assert!(matches!(
        service.submit(
            "session",
            CaseId::new(),
            command(),
            Sha256Digest::from_array([0; 32])
        ),
        Err(ApplicationError::InvalidSession)
    ));
    assert_eq!(clock.calls(), 0);
}

#[test]
fn changing_authenticated_actor_after_validation_prevents_commit() {
    let (_, actor) = identity(Role::Owner, 0);
    let case_id = CaseId::new();
    let cmd = command();
    let prep = preparation(case_id, actor.id);
    let digest = detail(case_id, actor.id, &cmd, values(), &prep.context)
        .snapshot
        .receipt
        .submission_digest;
    let mut auth = MockIdentity::new();
    let mut sequence = mockall::Sequence::new();
    let first = actor.clone();
    auth.expect_authenticate()
        .times(1)
        .in_sequence(&mut sequence)
        .return_once(move |_| Ok(first));
    auth.expect_authenticate()
        .times(1)
        .in_sequence(&mut sequence)
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
fn stage_profile_immutable_kind_and_exact_support_checks_precede_crypto() {
    for mode in 0..6 {
        let (identity, actor) = identity(Role::Owner, 1);
        let case_id = CaseId::new();
        let mut prep = preparation(case_id, actor.id);
        let mut cmd = command();
        match mode {
            0 => {
                if let application::cases::CurrentCaseAdministration::Recorded(ref mut admin) =
                    prep.context.administration
                {
                    admin.values = application::cases::CaseAdministrationValues::basic(
                        admin.values.metadata().clone(),
                    );
                    admin.values_digest = application::cases::case_administration_digest(
                        hasher().as_ref(),
                        &admin.values,
                    );
                    prep.context.stage = application::case_stages::CurrentCaseStage::Unregistered
                }
            }
            1 => {
                if let HearingChange::Schedule { ref mut values, .. } = cmd.change {
                    let mut input = values_input(values);
                    input.kind = HearingKind::Intermediate;
                    *values = HearingValues::new(input).unwrap();
                }
            }
            2 => {
                prep.base = Some(detail(case_id, actor.id, &cmd, values(), &prep.context));
            }
            3 => {
                let base = detail(case_id, actor.id, &cmd, values(), &prep.context);
                cmd = replacement(&base);
                if let HearingChange::Replace { ref mut values, .. } = cmd.change {
                    let mut input = values_input(values);
                    input.kind = HearingKind::Intermediate;
                    *values = HearingValues::new(input).unwrap();
                }
                prep.base = Some(base);
            }
            _ => {
                let record = crypto::processor().prepare("proof.pdf", b"proof").unwrap();
                trial(&mut prep, actor.id, &record);
                cmd = sentencing(&record);
                if mode == 5 {
                    let mut foreign = record;
                    foreign.id = domain::crypto::DocumentId::new();
                    prep.records = vec![foreign];
                }
            }
        }
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _| Ok(prep));
        let validator = Arc::new(Validator::default());
        let (service, _) = service(store, identity, validator.clone());
        assert!(
            service.prepare("session", case_id, cmd).is_err(),
            "mode{mode}"
        );
        assert_eq!(validator.calls.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn commit_failures_propagate_and_never_become_a_successful_reconciliation() {
    for error in [
        ApplicationError::Hearing(HearingError::OperationConflict),
        ApplicationError::Hearing(HearingError::ContextConflict),
        ApplicationError::StageSupportChanged,
        ApplicationError::Port("audit unavailable".into()),
    ] {
        let expected = match error {
            ApplicationError::StageSupportChanged => {
                ApplicationError::Hearing(HearingError::SupportChanged).to_string()
            }
            _ => error.to_string(),
        };
        let (identity, actor) = identity(Role::Owner, 2);
        let case_id = CaseId::new();
        let cmd = command();
        let prep = preparation(case_id, actor.id);
        let digest = detail(case_id, actor.id, &cmd, values(), &prep.context)
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

#[test]
fn foreign_detail_or_history_is_not_returned_even_when_its_receipt_is_valid() {
    for history in [false, true] {
        let (identity, actor) = identity(Role::Owner, 1);
        let case_id = CaseId::new();
        let other = CaseId::new();
        let cmd = command();
        let result = detail(other, actor.id, &cmd, values(), &context(other, actor.id));
        let mut store = MockStore::new();
        if history {
            store
                .expect_history()
                .times(1)
                .return_once(move |_, _, _, _, _| {
                    Ok(HearingHistoryPage {
                        revisions: vec![result],
                        has_more: false,
                        next_before_revision: None,
                    })
                });
        } else {
            store
                .expect_get()
                .times(1)
                .return_once(move |_, _, _, _, _| Ok(result));
        }
        let (service, _) = service(store, identity, Arc::new(Validator::default()));
        let result = if history {
            service
                .history(
                    "session",
                    case_id,
                    cmd.hearing_id,
                    HearingHistoryQuery::new(20, None).unwrap(),
                )
                .map(|_| ())
        } else {
            service
                .get("session", case_id, cmd.hearing_id, None)
                .map(|_| ())
        };
        assert!(matches!(
            result,
            Err(ApplicationError::Hearing(HearingError::StoredInconsistent(
                _
            )))
        ));
    }
}
