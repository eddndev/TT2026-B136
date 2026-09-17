#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[path = "document_format_support/mod.rs"]
mod format_support;
#[allow(dead_code)]
mod hearing_support;

use application::case_stages::StageSupportSnapshot;
use application::participants::DirectoryStatus;
use application::{
    documents::{StageDocumentFormat, StageFormatPolicy},
    hearings::*,
    ApplicationError,
};
use domain::{cases::CaseId, identity::Role};
use hearing_support::*;
use std::sync::{atomic::Ordering, Arc};

#[test]
fn sentencing_support_is_decrypted_validated_and_captured_before_reauthentication() {
    let producer = crypto::processor();
    let record = producer
        .seal(
            &producer
                .prepare("judgment.pdf", b"signed judgment")
                .unwrap(),
        )
        .unwrap();
    let command = sentencing(&record);
    let (_, actor) = identity(Role::Owner, 0);
    let case_id = CaseId::new();
    let mut prep = preparation(case_id, actor.id);
    trial(&mut prep, actor.id, &record);
    prep.records = vec![record.clone()];
    let expected = detail(
        case_id,
        actor.id,
        &command,
        command_values(&command),
        &prep.context,
    );
    let mut result = expected.clone();
    result.support = Some(StageSupportSnapshot {
        reference: support(&record).reference(),
        digest: record.digest,
        name: record.name.clone(),
        format: StageDocumentFormat::Pdf,
        policy: StageFormatPolicy::PdfDocxV1,
    });
    let expected_digest = result.snapshot.receipt.submission_digest;
    let (processor, observations) = format_support::processor();
    let validator = Arc::new(Validator::default());
    let mut auth = MockIdentity::new();
    let mut order = mockall::Sequence::new();
    let first = actor.clone();
    auth.expect_authenticate()
        .times(1)
        .in_sequence(&mut order)
        .return_once(move |_| Ok(first));
    let observed = observations.clone();
    let checked = validator.clone();
    auth.expect_authenticate()
        .times(1)
        .in_sequence(&mut order)
        .return_once(move |_| {
            let events = &observed.lock().unwrap().events;
            for event in [
                "unwrap",
                "open",
                "hash",
                "signature",
                "timestamp",
                "certificate",
            ] {
                assert_eq!(events.iter().filter(|v| **v == event).count(), 1);
            }
            assert_eq!(checked.calls.load(Ordering::SeqCst), 1);
            Ok(actor)
        });
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(prep));
    store
        .expect_commit()
        .times(1)
        .return_once(move |_, _, prepared| {
            assert_eq!(prepared.preparation().records, vec![record]);
            assert_eq!(prepared.formats(), &[StageDocumentFormat::Pdf]);
            assert!(prepared.scheduling_context().stage_digest.is_some());
            Ok(result)
        });
    let clock = Arc::new(CountingClock::default());
    let service = HearingService::new(
        Arc::new(store),
        Arc::new(auth),
        Arc::new(processor),
        validator,
        hasher(),
        clock.clone(),
    );
    assert_eq!(
        service
            .submit("session", case_id, command, expected_digest)
            .unwrap()
            .snapshot,
        expected.snapshot
    );
    assert_eq!(clock.calls(), 0);
}

#[test]
fn cancellation_preserves_old_support_and_participants_after_stage_change_without_crypto() {
    let producer = crypto::processor();
    let record = producer.prepare("old.pdf", b"historical support").unwrap();
    let mut original = sentencing(&record);
    let (identity, actor) = identity(Role::Owner, 2);
    let case_id = CaseId::new();
    let mut prep = preparation(case_id, actor.id);
    let current = prep.context.clone();
    trial(&mut prep, actor.id, &record);
    let participant = participant(case_id, 1, DirectoryStatus::Active);
    selected(&mut original, std::slice::from_ref(&participant));
    let mut base = detail(
        case_id,
        actor.id,
        &original,
        command_values(&original),
        &prep.context,
    );
    base.participants = vec![participant];
    base.support = Some(StageSupportSnapshot {
        reference: support(&record).reference(),
        digest: record.digest,
        name: record.name,
        format: StageDocumentFormat::Pdf,
        policy: StageFormatPolicy::PdfDocxV1,
    });
    let command = cancellation(&base);
    let expected = base.snapshot.values.clone();
    prep.context = current;
    prep.participants = base.participants.clone();
    prep.base = Some(base);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(prep));
    let (processor, observations) = format_support::processor();
    let validator = Arc::new(Validator::default());
    let service = HearingService::new(
        Arc::new(store),
        Arc::new(identity),
        Arc::new(processor),
        validator.clone(),
        hasher(),
        Arc::new(CountingClock::default()),
    );
    let draft = service.prepare("session", case_id, command).unwrap();
    assert_eq!(draft.values, expected);
    assert_eq!(draft.result_revision.get(), 2);
    assert!(observations.lock().unwrap().events.is_empty());
    assert_eq!(validator.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn new_selection_requires_active_exact_case_revisions_and_retained_snapshots_do_not_change() {
    for mode in 0..5 {
        let (identity, actor) = identity(Role::Owner, 1);
        let case_id = CaseId::new();
        let mut command = command();
        let mut prep = preparation(case_id, actor.id);
        let participant = participant(case_id, 1, DirectoryStatus::Active);
        selected(&mut command, std::slice::from_ref(&participant));
        prep.participants = vec![participant.clone()];
        match mode {
            0 => prep.participants[0].overview.directory_status = DirectoryStatus::Archived,
            1 => prep.participants[0].overview.case_id = CaseId::new(),
            2 => {
                prep.participants[0].overview.revision =
                    domain::participants::ParticipantRevision::new(2).unwrap()
            }
            3 => prep.participants.push(participant),
            _ => {
                let mut base = detail(
                    case_id,
                    actor.id,
                    &command,
                    command_values(&command),
                    &prep.context,
                );
                base.participants = vec![participant];
                command = replacement(&base);
                prep.base = Some(base);
                prep.participants[0].values_digest =
                    domain::crypto::Sha256Digest::from_array([99; 32]);
            }
        }
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _| Ok(prep));
        let (service, _) = service(store, identity, Arc::new(Validator::default()));
        assert!(
            service.prepare("session", case_id, command).is_err(),
            "mode{mode}"
        );
    }
}

#[test]
fn selected_participants_are_normalized_to_canonical_order_and_reused_as_exact_history() {
    let (identity, actor) = identity(Role::Owner, 2);
    let case_id = CaseId::new();
    let mut prep = preparation(case_id, actor.id);
    let mut command = command();
    let participants = vec![
        participant(case_id, 7, DirectoryStatus::Active),
        participant(case_id, 1, DirectoryStatus::Active),
    ];
    selected(&mut command, &participants);
    prep.participants = participants.clone();
    let mut expected = detail(
        case_id,
        actor.id,
        &command,
        command_values(&command),
        &prep.context,
    );
    expected.participants = participants.into_iter().rev().collect();
    let digest = expected.snapshot.receipt.submission_digest;
    let result = expected.clone();
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(prep));
    store
        .expect_commit()
        .times(1)
        .return_once(move |_, _, prepared| {
            assert_eq!(prepared.preparation().participants, result.participants);
            Ok(result)
        });
    let (service, _) = service(store, identity, Arc::new(Validator::default()));
    assert_eq!(
        service.submit("session", case_id, command, digest).unwrap(),
        expected
    );
}

#[test]
fn changed_support_hash_is_rejected_before_crypto_and_parser() {
    let producer = crypto::processor();
    let record = producer.prepare("proof.pdf", b"proof").unwrap();
    let command = sentencing(&record);
    let (identity, actor) = identity(Role::Owner, 1);
    let case_id = CaseId::new();
    let mut prep = preparation(case_id, actor.id);
    trial(&mut prep, actor.id, &record);
    let mut changed = record;
    changed.digest = domain::crypto::Sha256Digest::from_array([9; 32]);
    prep.records = vec![changed];
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(prep));
    let validator = Arc::new(Validator::default());
    let (service, _) = service(store, identity, validator.clone());
    assert!(matches!(
        service.prepare("session", case_id, command),
        Err(ApplicationError::Hearing(
            HearingError::SupportDigestMismatch
        ))
    ));
    assert_eq!(validator.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn format_rejection_is_reported_with_a_hearing_error() {
    let record = crypto::processor().prepare("bad.pdf", b"bad").unwrap();
    let command = sentencing(&record);
    let (identity, actor) = identity(Role::Owner, 1);
    let case_id = CaseId::new();
    let mut prep = preparation(case_id, actor.id);
    trial(&mut prep, actor.id, &record);
    prep.records = vec![record];
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(prep));
    let validator = Arc::new(Validator {
        failure: true,
        ..Default::default()
    });
    let (service, _) = service(store, identity, validator);
    assert!(matches!(
        service.prepare("session", case_id, command),
        Err(ApplicationError::Hearing(
            HearingError::SupportFormatRejected
        ))
    ));
}
