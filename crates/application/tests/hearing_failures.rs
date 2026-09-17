#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
mod hearing_support;

use application::{hearings::*, ApplicationError};
use domain::{cases::CaseId, identity::Role};
use hearing_support::*;
use std::sync::Arc;

#[test]
fn invalid_prepared_context_scope_digest_or_revision_never_commits() {
    for mode in 0..6 {
        let (identity, actor) = identity(Role::Owner, 1);
        let case_id = CaseId::new();
        let command = command();
        let mut prep = preparation(case_id, actor.id);
        match mode {
            0 => prep.context.case_id = CaseId::new(),
            1 => {
                if let application::cases::CurrentCaseAdministration::Recorded(ref mut value) =
                    prep.context.administration
                {
                    value.case_id = CaseId::new()
                }
            }
            2 => {
                if let application::cases::CurrentCaseAdministration::Recorded(ref mut value) =
                    prep.context.administration
                {
                    value.values_digest = domain::crypto::Sha256Digest::from_array([99; 32])
                }
            }
            3 => {
                if let application::cases::CurrentCaseAdministration::Recorded(ref mut value) =
                    prep.context.administration
                {
                    value.revision = domain::case_administration::CaseRevision::new(2).unwrap()
                }
            }
            4 => prep.context.stage = application::case_stages::CurrentCaseStage::Unregistered,
            _ => {
                if let application::cases::CurrentCaseAdministration::Recorded(ref mut value) =
                    prep.context.administration
                {
                    value.values = value
                        .values
                        .with_status(domain::case_administration::CaseAdministrativeStatus::Closed);
                    value.values_digest = application::cases::case_administration_digest(
                        hasher().as_ref(),
                        &value.values,
                    )
                }
            }
        }
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _| Ok(prep));
        let (service, _) = service(store, identity, Arc::new(Validator::default()));
        let result = service.prepare("session", case_id, command);
        assert!(result.is_err(), "mode {mode}");
    }
}

#[test]
fn stale_missing_cancelled_foreign_and_replayed_bases_are_rejected() {
    for mode in 0..7 {
        let (identity, actor) = identity(Role::Owner, 1);
        let case_id = CaseId::new();
        let mut prep = preparation(case_id, actor.id);
        let mut base = detail(case_id, actor.id, &command(), values(), &prep.context);
        let mut command = replacement(&base);
        match mode {
            0 => base.snapshot.id = HearingId::new(),
            1 => base.snapshot.case_id = CaseId::new(),
            2 => base.snapshot.revision = HearingRevision::new(9).unwrap(),
            3 => base.snapshot.status = HearingStatus::Cancelled,
            4 => base.snapshot.values_digest = domain::crypto::Sha256Digest::from_array([44; 32]),
            5 => command.operation_id = base.snapshot.receipt.operation_id,
            _ => {}
        }
        prep.base = (mode != 6).then_some(base);
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _| Ok(prep));
        let (service, _) = service(store, identity, Arc::new(Validator::default()));
        assert!(
            service.prepare("session", case_id, command).is_err(),
            "mode {mode}"
        );
    }
}

#[test]
fn exhausted_revision_rejects_before_reading_storage() {
    let (identity, _) = identity(Role::Owner, 1);
    let command = HearingCommand {
        operation_id: HearingOperationId::new(),
        hearing_id: HearingId::new(),
        change: HearingChange::Cancel {
            expected_revision: HearingRevision::new(u32::MAX).unwrap(),
            reason: HearingNote::new("Reason").unwrap(),
        },
    };
    let (service, _) = service(MockStore::new(), identity, Arc::new(Validator::default()));
    assert!(matches!(
        service.prepare("session", CaseId::new(), command),
        Err(ApplicationError::Hearing(HearingError::RevisionExhausted))
    ));
}

#[test]
fn expired_session_after_preparation_prevents_commit() {
    let (_, actor) = identity(Role::Owner, 0);
    let case_id = CaseId::new();
    let command = command();
    let prep = preparation(case_id, actor.id);
    let digest = detail(case_id, actor.id, &command, values(), &prep.context)
        .snapshot
        .receipt
        .submission_digest;
    let mut identity = MockIdentity::new();
    let mut sequence = mockall::Sequence::new();
    identity
        .expect_authenticate()
        .times(1)
        .in_sequence(&mut sequence)
        .return_once(move |_| Ok(actor));
    identity
        .expect_authenticate()
        .times(1)
        .in_sequence(&mut sequence)
        .return_once(|_| Err(ApplicationError::InvalidSession));
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(prep));
    let (service, _) = service(store, identity, Arc::new(Validator::default()));
    assert!(matches!(
        service.submit("session", case_id, command, digest),
        Err(ApplicationError::InvalidSession)
    ));
}

#[test]
fn submission_digest_mismatch_prevents_commit() {
    let (identity, actor) = identity(Role::Owner, 1);
    let case_id = CaseId::new();
    let prep = preparation(case_id, actor.id);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(prep));
    let (service, _) = service(store, identity, Arc::new(Validator::default()));
    assert!(matches!(
        service.submit(
            "session",
            case_id,
            command(),
            domain::crypto::Sha256Digest::from_array([99; 32])
        ),
        Err(ApplicationError::Hearing(HearingError::SubmissionMismatch))
    ));
}

#[test]
fn admission_errors_from_storage_are_mapped_without_changing_unrelated_failures() {
    for (error, expected) in [
        (
            ApplicationError::StageSupportTooLarge,
            HearingError::SupportTooLarge,
        ),
        (
            ApplicationError::StageSupportFormatRejected,
            HearingError::SupportFormatRejected,
        ),
        (
            ApplicationError::StageSupportValidationLimit,
            HearingError::SupportValidationLimit,
        ),
        (
            ApplicationError::StageSupportDigestMismatch,
            HearingError::SupportDigestMismatch,
        ),
        (
            ApplicationError::StageSupportChanged,
            HearingError::SupportChanged,
        ),
        (
            HearingError::OperationConflict.into(),
            HearingError::OperationConflict,
        ),
    ] {
        let (identity, _) = identity(Role::Owner, 1);
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _| Err(error));
        let (service, _) = service(store, identity, Arc::new(Validator::default()));
        assert_eq!(
            service
                .prepare("session", CaseId::new(), command())
                .unwrap_err()
                .to_string(),
            ApplicationError::Hearing(expected).to_string()
        );
    }
}

#[test]
fn cancellation_cannot_be_prepared_against_a_missing_or_rolled_back_administration() {
    for missing in [true, false] {
        let (identity, actor) = identity(Role::Owner, 1);
        let case_id = CaseId::new();
        let mut prep = preparation(case_id, actor.id);
        let mut base = detail(case_id, actor.id, &command(), values(), &prep.context);
        if missing {
            prep.context.administration = application::cases::CurrentCaseAdministration::Unrevised(
                domain::cases::CaseMetadata::new("Legacy", "REF").unwrap(),
            );
            prep.context.stage = application::case_stages::CurrentCaseStage::Unregistered;
        } else {
            let mut newer_context = prep.context.clone();
            if let application::cases::CurrentCaseAdministration::Recorded(ref mut admin) =
                newer_context.administration
            {
                admin.revision = domain::case_administration::CaseRevision::new(2).unwrap();
            }
            let mut original = command();
            if let HearingChange::Schedule {
                ref mut context, ..
            } = original.change
            {
                context.case_revision = domain::case_administration::CaseRevision::new(2).unwrap();
            }
            base = detail(case_id, actor.id, &original, values(), &newer_context);
        }
        let command = cancellation(&base);
        prep.base = Some(base);
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _| Ok(prep));
        let (service, _) = service(store, identity, Arc::new(Validator::default()));
        assert!(service.prepare("session", case_id, command).is_err());
    }
}
