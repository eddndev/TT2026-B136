#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
mod hearing_result_support;
#[allow(dead_code)]
mod hearing_support;

#[path = "document_format_support/mod.rs"]
mod format_support;
use application::{
    case_stages::StageSupportSnapshot,
    documents::{DocumentRecord, StageDocumentFormat, StageFormatPolicy},
    hearing_results::*,
    ApplicationError,
};
use domain::{cases::CaseId, crypto::DocumentVersionRef, identity::Role};
use hearing_result_support::*;
use std::sync::{atomic::Ordering, Arc};
fn supported(command: &mut HearingResultCommand, record: &DocumentRecord) {
    match &mut command.change {
        HearingResultChange::Record { values, .. }
        | HearingResultChange::Correct { values, .. } => {
            let mut input = values_input(values);
            input.provenance = HearingResultProvenance::new(
                HearingResultProvenanceKind::OralReference,
                Some(HearingResultReference::new("Declared oral communication").unwrap()),
                Some(HearingResultSupportRef::new(
                    DocumentVersionRef {
                        id: record.id,
                        version: record.version,
                    },
                    record.digest,
                )),
            )
            .unwrap();
            *values = HearingResultValues::new(input).unwrap();
        }
        _ => panic!("withdrawal preserves its source"),
    }
}
fn projection(record: &DocumentRecord) -> StageSupportSnapshot {
    StageSupportSnapshot {
        reference: DocumentVersionRef {
            id: record.id,
            version: record.version,
        },
        digest: record.digest,
        name: record.name.clone(),
        format: StageDocumentFormat::Pdf,
        policy: StageFormatPolicy::PdfDocxV1,
    }
}
#[test]
fn correction_revalidates_identical_support_before_refreshed_authentication() {
    let producer = crypto::processor();
    let record = producer
        .seal(
            &producer
                .prepare("minutes.pdf", b"declared oral result")
                .unwrap(),
        )
        .unwrap();
    let (_, actor) = identity(Role::Owner, 0);
    let case_id = CaseId::new();
    let mut prep = preparation(case_id, actor.id);
    let mut original = command(&prep);
    supported(&mut original, &record);
    let mut base = detail(actor.id, &original, &prep);
    base.support = Some(projection(&record));
    let cmd = correction(&base);
    prep.base = Some(base);
    prep.records = vec![record.clone()];
    let (processor, observed) = format_support::processor();
    let validator = Arc::new(Validator::default());
    let mut auth = MockIdentity::new();
    let mut sequence = mockall::Sequence::new();
    let first = actor.clone();
    auth.expect_authenticate()
        .times(1)
        .in_sequence(&mut sequence)
        .return_once(move |_| Ok(first));
    let observation = observed.clone();
    let checked = validator.clone();
    auth.expect_authenticate()
        .times(1)
        .in_sequence(&mut sequence)
        .return_once(move |_| {
            let events = &observation.lock().unwrap().events;
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
    let service = HearingResultService::new(
        Arc::new(store),
        Arc::new(auth),
        Arc::new(processor),
        validator,
        hasher(),
        Arc::new(CountingClock::default()),
    );
    let draft = service.prepare("session", case_id, cmd).unwrap();
    assert_eq!(draft.support, Some(projection(&record)));
    assert_eq!(
        draft.values.provenance().kind(),
        HearingResultProvenanceKind::OralReference
    );
}
#[test]
fn withdrawal_preserves_admission_and_skips_crypto_parser_and_preliminary_clock() {
    let record = crypto::processor()
        .prepare("old.pdf", b"historical source")
        .unwrap();
    let (identity, actor) = identity(Role::Owner, 2);
    let case_id = CaseId::new();
    let mut prep = preparation(case_id, actor.id);
    let mut original = command(&prep);
    supported(&mut original, &record);
    let mut base = detail(actor.id, &original, &prep);
    base.support = Some(projection(&record));
    let cmd = withdrawal(&base);
    prep.base = Some(base);
    let (processor, observed) = format_support::processor();
    let validator = Arc::new(Validator::default());
    let clock = Arc::new(CountingClock::default());
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(prep));
    let service = HearingResultService::new(
        Arc::new(store),
        Arc::new(identity),
        Arc::new(processor),
        validator.clone(),
        hasher(),
        clock.clone(),
    );
    let draft = service.prepare("session", case_id, cmd).unwrap();
    assert_eq!(draft.support, Some(projection(&record)));
    assert!(observed.lock().unwrap().events.is_empty());
    assert_eq!(validator.calls.load(Ordering::SeqCst), 0);
    assert_eq!(clock.calls(), 0);
}
#[test]
fn support_mismatch_and_format_rejection_prevent_a_draft() {
    for mode in 0..3 {
        let record = crypto::processor()
            .prepare("support.pdf", b"source")
            .unwrap();
        let (identity, actor) = identity(Role::Owner, 1);
        let case_id = CaseId::new();
        let mut prep = preparation(case_id, actor.id);
        let mut cmd = command(&prep);
        supported(&mut cmd, &record);
        prep.records = vec![record];
        if mode == 0 {
            prep.records[0].digest = domain::crypto::Sha256Digest::from_array([99; 32]);
        }
        if mode == 1 {
            prep.records.clear();
        }
        let validator = Arc::new(Validator {
            failure: mode == 2,
            ..Default::default()
        });
        let mut store = MockStore::new();
        store
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _, _| Ok(prep));
        let (service, _) = service(store, identity, validator.clone());
        let result = service.prepare("session", case_id, cmd);
        match mode {
            0 => assert!(matches!(
                result,
                Err(ApplicationError::HearingResult(
                    HearingResultError::SupportDigestMismatch
                ))
            )),
            1 => assert!(matches!(
                result,
                Err(ApplicationError::HearingResult(
                    HearingResultError::StoredInconsistent(_)
                ))
            )),
            _ => assert!(matches!(
                result,
                Err(ApplicationError::HearingResult(
                    HearingResultError::SupportFormatRejected
                ))
            )),
        }
        assert_eq!(
            validator.calls.load(Ordering::SeqCst),
            usize::from(mode == 2)
        );
    }
}
#[test]
fn corrected_retained_attendee_projection_cannot_silently_change_exact_values() {
    let (identity, actor) = identity(Role::Owner, 1);
    let case_id = CaseId::new();
    let mut prep = preparation(case_id, actor.id);
    let mut original = command(&prep);
    let attendee = HearingResultAttendeeSnapshot {
        participant: hearing_support::participant(
            case_id,
            1,
            application::participants::DirectoryStatus::Archived,
        ),
        subject_digest: None,
    };
    if let HearingResultChange::Record { values, .. } = &mut original.change {
        let mut input = values_input(values);
        input.attendees = vec![HearingResultAttendee::new(
            attendee.participant.overview.id,
            attendee.participant.overview.revision,
            HearingResultCapacity::new("Declared witness").unwrap(),
            None,
        )];
        *values = HearingResultValues::new(input).unwrap();
    }
    prep.attendees = vec![attendee];
    let base = detail(actor.id, &original, &prep);
    let cmd = correction(&base);
    prep.base = Some(base);
    prep.attendees[0].participant.values_digest =
        domain::crypto::Sha256Digest::from_array([99; 32]);
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(prep));
    let (service, _) = service(store, identity, Arc::new(Validator::default()));
    assert!(matches!(
        service.prepare("session", case_id, cmd),
        Err(ApplicationError::HearingResult(
            HearingResultError::StoredInconsistent(_)
        ))
    ));
}
#[test]
fn later_incomplete_administration_is_observed_without_profile_or_stage_cas() {
    let (identity, actor) = identity(Role::Owner, 2);
    let case_id = CaseId::new();
    let mut prep = preparation(case_id, actor.id);
    let cmd = command(&prep);
    if let application::cases::CurrentCaseAdministration::Recorded(ref mut admin) =
        prep.administration
    {
        admin.revision = domain::case_administration::CaseRevision::new(2).unwrap();
        admin.values =
            application::cases::CaseAdministrationValues::basic(admin.values.metadata().clone());
        admin.values_digest =
            application::cases::case_administration_digest(hasher().as_ref(), &admin.values);
    }
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(prep));
    let (service, _) = service(store, identity, Arc::new(Validator::default()));
    let draft = service.prepare("session", case_id, cmd).unwrap();
    assert_eq!(draft.observed_administration.revision().unwrap().get(), 2);
    assert!(draft.observed_administration.values().profile().is_none());
}
