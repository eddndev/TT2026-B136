use crate::hearing_derived_deadline_capture_support::*;
use crate::hearing_derived_deadline_support::*;
use crate::hearing_result_support::{hasher, values_input};
use application::{
    case_stages::StageSupportSnapshot,
    deadline_reevaluation::DependencyFamily,
    documents::{StageDocumentFormat, StageFormatPolicy},
    hearing_derived_deadlines::*,
    hearing_results::*,
};
use domain::{
    case_administration::CaseRevision,
    cases::CaseId,
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest},
    hearings::{HearingId, HearingTime},
    identity::UserId,
};
use time::{Date, Duration, Month, UtcOffset};
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
enum SourceMismatch {
    AuthorIdentity,
    AuthorEmail,
    Operation,
    ResultIdentity,
    HearingIdentity,
    Case,
    AdministrationRevision,
    AdministrationDigest,
    AnchorTime,
    AnchorOffset,
    Values,
}

#[test]
fn a_valid_but_different_recorded_source_cannot_finalize_the_reviewed_instruction() {
    let fixture = fixture();
    let reviewed = fixture.prepare().unwrap();
    let original = recorded_source(
        &fixture,
        crate::case_support::instant() + Duration::hours(1),
    );
    for field in [
        SourceMismatch::AuthorIdentity,
        SourceMismatch::AuthorEmail,
        SourceMismatch::Operation,
        SourceMismatch::ResultIdentity,
        SourceMismatch::HearingIdentity,
        SourceMismatch::Case,
        SourceMismatch::AdministrationRevision,
        SourceMismatch::AdministrationDigest,
        SourceMismatch::AnchorTime,
        SourceMismatch::AnchorOffset,
        SourceMismatch::Values,
    ] {
        let mut source = original.clone();
        match field {
            SourceMismatch::AuthorIdentity => source.snapshot.recorded_by.id = UserId::new(),
            SourceMismatch::AuthorEmail => {
                source.snapshot.recorded_by.email = "other@example.com".into()
            }
            SourceMismatch::Operation => {
                source.snapshot.receipt.operation_id = HearingResultOperationId::new()
            }
            SourceMismatch::ResultIdentity => source.snapshot.id = HearingResultId::new(),
            SourceMismatch::HearingIdentity => {
                let id = HearingId::new();
                source.snapshot.hearing_id = id;
                source.snapshot.anchor.hearing_id = id;
                source.anchor.reference.hearing_id = id;
            }
            SourceMismatch::Case => source.snapshot.case_id = CaseId::new(),
            SourceMismatch::AdministrationRevision => {
                source.snapshot.recorded_administration_revision = CaseRevision::new(2).unwrap();
            }
            SourceMismatch::AdministrationDigest => {
                let digest = Sha256Digest::from_array([13; 32]);
                source.snapshot.recorded_administration_digest = digest;
                source.anchor.scheduling_context.administration_digest = digest;
            }
            SourceMismatch::AnchorTime => {
                source.anchor.scheduled_at =
                    HearingTime::new(source.anchor.scheduled_at.value() + Duration::hours(1))
                        .unwrap();
            }
            SourceMismatch::AnchorOffset => {
                source.anchor.scheduled_at = HearingTime::new(
                    source
                        .anchor
                        .scheduled_at
                        .value()
                        .to_offset(UtcOffset::from_hms(-6, 0, 0).unwrap()),
                )
                .unwrap();
            }
            SourceMismatch::Values => {
                let mut values = values_input(&source.snapshot.values);
                values.summary = HearingResultText::new("A different actual declaration").unwrap();
                source.snapshot.values = HearingResultValues::new(values).unwrap();
            }
        }
        resign_recorded_source(&mut source);
        // An event matching the alternative source cannot authorize replacing the draft.
        let event = source_event(&source);
        assert!(
            finalize_hearing_derived_deadline(hasher().as_ref(), &reviewed, source, event,)
                .is_err(),
            "{field:?}"
        );
    }
}

#[test]
fn corrupt_source_receipts_revisions_and_undeclared_projections_are_rejected() {
    let fixture = fixture();
    let reviewed = fixture.prepare().unwrap();
    let original = recorded_source(
        &fixture,
        crate::case_support::instant() + Duration::hours(1),
    );
    for field in [
        "values_digest",
        "receipt_digest",
        "revision",
        "support",
        "withdrawn",
    ] {
        let mut source = original.clone();
        match field {
            "values_digest" => source.snapshot.values_digest = Sha256Digest::from_array([4; 32]),
            "receipt_digest" => {
                source.snapshot.receipt.submission_digest = Sha256Digest::from_array([5; 32])
            }
            "revision" => source.snapshot.revision = HearingResultRevision::new(2).unwrap(),
            "support" => {
                source.support = Some(StageSupportSnapshot {
                    reference: DocumentVersionRef {
                        id: DocumentId::new(),
                        version: DocumentVersion::initial(),
                    },
                    digest: Sha256Digest::from_array([6; 32]),
                    name: "Unreviewed support".into(),
                    format: StageDocumentFormat::Pdf,
                    policy: StageFormatPolicy::PdfDocxV1,
                })
            }
            _ => source.snapshot.status = HearingResultStatus::Withdrawn,
        }
        let event = source_event(&source);
        assert!(
            finalize_hearing_derived_deadline(hasher().as_ref(), &reviewed, source, event,)
                .is_err(),
            "{field}"
        );
    }
}

#[test]
fn source_event_must_match_every_field_and_fit_the_persistent_sequence_range() {
    let fixture = fixture();
    let reviewed = fixture.prepare().unwrap();
    let source = recorded_source(
        &fixture,
        crate::case_support::instant() + Duration::hours(1),
    );
    let exact = source_event(&source);
    for field in [
        "family",
        "root",
        "revision",
        "zero_revision",
        "case",
        "missing_case",
        "hearing",
        "missing_hearing",
        "operation",
        "zero_sequence",
        "overflow_sequence",
    ] {
        let mut event = exact;
        match field {
            "family" => event.family = DependencyFamily::Resolution,
            "root" => event.source_id = Uuid::new_v4(),
            "revision" => event.revision = 2,
            "zero_revision" => event.revision = 0,
            "case" => event.case_id = Some(CaseId::new()),
            "missing_case" => event.case_id = None,
            "hearing" => event.hearing_id = Some(Uuid::new_v4()),
            "missing_hearing" => event.hearing_id = None,
            "operation" => event.operation_id = Uuid::new_v4(),
            "zero_sequence" => event.sequence = 0,
            _ => event.sequence = (i64::MAX as u64) + 1,
        }
        assert!(
            finalize_hearing_derived_deadline(hasher().as_ref(), &reviewed, source.clone(), event,)
                .is_err(),
            "{field}"
        );
    }
}

#[test]
fn event_sequence_is_bound_to_compound_capture_without_changing_the_deadline_receipt() {
    let fixture = fixture();
    let reviewed = fixture.prepare().unwrap();
    let source = recorded_source(
        &fixture,
        crate::case_support::instant() + Duration::hours(1),
    );
    let mut event = source_event(&source);
    let first =
        finalize_hearing_derived_deadline(hasher().as_ref(), &reviewed, source.clone(), event)
            .unwrap();
    event.sequence += 1;
    let second =
        finalize_hearing_derived_deadline(hasher().as_ref(), &reviewed, source, event).unwrap();
    // These are independent event-stream fixtures, not two inserts into one log.
    assert_eq!(first.deadline().receipt, second.deadline().receipt);
    assert_ne!(first.capture_digest(), second.capture_digest());
    assert_ne!(
        hearing_derived_deadline_capture_bytes(&first).unwrap(),
        hearing_derived_deadline_capture_bytes(&second).unwrap(),
    );
}

#[test]
fn capture_cannot_predate_the_declared_result_or_leave_the_supported_civil_years() {
    let fixture = fixture();
    let reviewed = fixture.prepare().unwrap();
    let original = recorded_source(
        &fixture,
        crate::case_support::instant() + Duration::hours(1),
    );
    for recorded_at in [
        original.snapshot.values.event_time().lower_bound() - Duration::seconds(1),
        Date::from_calendar_date(0, Month::January, 1)
            .unwrap()
            .midnight()
            .assume_utc(),
        Date::from_calendar_date(9999, Month::December, 31)
            .unwrap()
            .with_hms(23, 59, 59)
            .unwrap()
            .assume_offset(UtcOffset::from_hms(-1, 0, 0).unwrap()),
    ] {
        let mut source = original.clone();
        source.snapshot.recorded_at = recorded_at;
        let event = source_event(&source);
        assert!(
            finalize_hearing_derived_deadline(hasher().as_ref(), &reviewed, source, event,)
                .is_err()
        );
    }
}
