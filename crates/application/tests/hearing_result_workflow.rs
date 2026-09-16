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
use domain::{cases::CaseId, identity::Role};
use hearing_result_support::*;
use std::sync::{atomic::Ordering, Arc};
#[test]
fn stateless_preparation_preserves_exact_cancelled_anchor_without_invented_attendance() {
    let (identity, actor) = identity(Role::Owner, 2);
    let case_id = CaseId::new();
    let mut prep = preparation(case_id, actor.id);
    let cancel = hearing_support::cancellation(&prep.anchor);
    prep.anchor = hearing_support::detail(
        case_id,
        actor.id,
        &cancel,
        prep.anchor.snapshot.values.clone(),
        &hearing_support::context(case_id, actor.id),
    );
    let command = command(&prep);
    let source = anchor(&prep.anchor);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |id, case, _, _| {
            assert_eq!((id, case), (actor.id, case_id));
            Ok(prep)
        });
    let validator = Arc::new(Validator::default());
    let (service, clock) = service(store, identity, validator.clone());
    let draft = service
        .prepare("session", case_id, command.clone())
        .unwrap();
    assert_eq!(draft.case_id, case_id);
    assert_eq!(draft.actor, actor.id);
    assert_eq!(draft.command, command);
    assert_eq!(draft.anchor, source);
    assert!(draft.attendees.is_empty());
    assert_eq!(draft.values, values());
    assert_eq!(draft.result_revision.get(), 1);
    assert_eq!(validator.calls.load(Ordering::SeqCst), 0);
    assert_eq!(clock.calls(), 1);
}
#[test]
fn submit_reprepares_reauthenticates_and_checks_the_exact_committed_receipt() {
    let (identity, actor) = identity(Role::Litigator, 2);
    let case_id = CaseId::new();
    let prep = preparation(case_id, actor.id);
    let cmd = command(&prep);
    let result = detail(actor.id, &cmd, &prep);
    let expected = result.clone();
    let digest = result.snapshot.receipt.submission_digest;
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(prep));
    store
        .expect_commit()
        .times(1)
        .return_once(move |id, case, prepared| {
            assert_eq!((id, case), (actor.id, case_id));
            assert_eq!(prepared.submission_digest(), digest);
            assert!(prepared.formats().is_empty());
            Ok(result)
        });
    let (service, clock) = service(store, identity, Arc::new(Validator::default()));
    assert_eq!(
        service.submit("session", case_id, cmd, digest).unwrap(),
        expected
    );
    assert_eq!(clock.calls(), 1);
}
#[test]
fn same_values_correction_still_produces_a_new_explicit_revision() {
    let (identity, actor) = identity(Role::Owner, 2);
    let case_id = CaseId::new();
    let mut prep = preparation(case_id, actor.id);
    let base = detail(actor.id, &command(&prep), &prep);
    let cmd = correction(&base);
    prep.base = Some(base);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(prep));
    let (service, _) = service(store, identity, Arc::new(Validator::default()));
    let draft = service.prepare("session", case_id, cmd).unwrap();
    assert_eq!(draft.result_revision.get(), 2);
    assert_eq!(draft.values, values());
}
#[test]
fn historical_archived_attendees_are_explicit_and_sorted_without_current_directory_checks() {
    let (identity, actor) = identity(Role::Owner, 2);
    let case_id = CaseId::new();
    let mut prep = preparation(case_id, actor.id);
    let mut cmd = command(&prep);
    let attendees: Vec<_> = [2, 1]
        .map(|id| HearingResultAttendeeSnapshot {
            participant: hearing_support::participant(
                case_id,
                id,
                application::participants::DirectoryStatus::Archived,
            ),
            subject_digest: None,
        })
        .into();
    let mut input = values_input(&values());
    input.attendees = attendees
        .iter()
        .map(|a| {
            HearingResultAttendee::new(
                a.participant.overview.id,
                a.participant.overview.revision,
                HearingResultCapacity::new("Declared capacity").unwrap(),
                None,
            )
        })
        .collect();
    if let HearingResultChange::Record { values, .. } = &mut cmd.change {
        *values = HearingResultValues::new(input).unwrap();
    }
    prep.attendees = attendees;
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(prep));
    let (service, _) = service(store, identity, Arc::new(Validator::default()));
    let draft = service.prepare("session", case_id, cmd).unwrap();
    assert_eq!(
        draft.attendees[0]
            .participant
            .overview
            .id
            .as_uuid()
            .as_u128(),
        1
    );
    assert_eq!(draft.values.attendees().len(), 2);
}
#[test]
fn paralegal_is_read_only_and_client_is_denied_before_store_and_clock() {
    let case_id = CaseId::new();
    let (identity, actor) = identity(Role::Paralegal, 2);
    let cmd = command(&preparation(case_id, actor.id));
    let (writer, clock) = service(MockStore::new(), identity, Arc::new(Validator::default()));
    assert!(matches!(
        writer.prepare("session", case_id, cmd.clone()),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(matches!(
        writer.submit(
            "session",
            case_id,
            cmd.clone(),
            domain::crypto::Sha256Digest::from_array([0; 32])
        ),
        Err(ApplicationError::PermissionDenied)
    ));
    assert_eq!(clock.calls(), 0);
    let (identity, _) = hearing_result_support::identity(Role::Client, 3);
    let (reader, clock) = service(MockStore::new(), identity, Arc::new(Validator::default()));
    assert!(matches!(
        reader.get("session", case_id, cmd.hearing_id, cmd.result_id, None),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(matches!(
        reader.list(
            "session",
            case_id,
            cmd.hearing_id,
            HearingResultQuery::new(20, None, HearingResultStatusFilter::All).unwrap()
        ),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(matches!(
        reader.history(
            "session",
            case_id,
            cmd.hearing_id,
            cmd.result_id,
            HearingResultHistoryQuery::new(10, None).unwrap()
        ),
        Err(ApplicationError::PermissionDenied)
    ));
    assert_eq!(clock.calls(), 0);
}
