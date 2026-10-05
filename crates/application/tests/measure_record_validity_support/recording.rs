use application::cases::CaseRevision;
use application::precautionary_hearings::PrecautionaryContext;
use domain::identity::{Role, UserId};
use time::{Date, Duration, Month, UtcOffset};
use uuid::Uuid;

use crate::validity_support::*;

#[test]
fn mark_records_the_actual_owner_or_litigator_and_exact_reason() {
    for role in [Role::Owner, Role::Litigator] {
        let mut fixture = marking(RecordFixture::initial());
        fixture.actor.id = UserId::from_uuid(Uuid::from_u128(900));
        fixture.actor.email = "error-recorder@example.test".into();
        fixture.actor.role = role;
        fixture.command.reason = note("Subject selection was copied from another record");
        let marked = capture(&fixture);
        assert_eq!(marked.review.actor, fixture.actor);
        assert_eq!(marked.records[0].actor, fixture.actor);
        assert_eq!(marked.review.command.reason, fixture.command.reason);
        assert_eq!(
            marked.review.result.judicial_origin,
            fixture.history.judicial.groups[0].capture.measures[0]
                .result
                .origin
        );
    }
}

#[test]
fn mark_preparation_requires_recording_role_and_clean_historical_actor_text() {
    for role in [Role::Paralegal, Role::Client] {
        let mut fixture = marking(RecordFixture::initial());
        fixture.actor.role = role;
        assert!(prepare(&fixture).is_err());
    }
    for email in ["", " leading", "trailing ", "control\ninside"] {
        let mut fixture = marking(RecordFixture::initial());
        fixture.actor.email = email.into();
        assert!(prepare(&fixture).is_err());
    }
}

#[test]
fn changing_only_the_required_reason_changes_the_complete_receipt_commitment() {
    let fixture = marking(RecordFixture::initial());
    let first = capture(&fixture);
    let mut changed = fixture.clone();
    changed.command.reason = note("Correct source is associated with another subject");
    let second = capture(&changed);
    assert_eq!(first.review.result, second.review.result);
    assert_ne!(
        first.review.submission_digest,
        second.review.submission_digest
    );
    assert_ne!(first.review.review_digest, second.review.review_digest);
    assert_ne!(
        first.records[0].capture_digest,
        second.records[0].capture_digest
    );
    assert_ne!(first.capture_digest, second.capture_digest);
}

#[test]
fn mark_after_correction_uses_the_effective_record_and_new_context_clock_floor() {
    let original = RecordFixture::initial();
    let corrected = original.capture();
    let mut fixture = marking(RecordFixture::next(&corrected, &original.history, 1));
    let mut material = fixture.context.material().clone();
    material.administration.revision = CaseRevision::new(2).unwrap();
    material.administration.changed_at = fixture.recorded_at + Duration::seconds(10);
    let floor = material.administration.changed_at;
    fixture.context = PrecautionaryContext::new(&Hasher, material).unwrap();
    fixture.command.context = expectation(&fixture.context);
    for before in [
        corrected.recorded_at - Duration::nanoseconds(1),
        floor - Duration::nanoseconds(1),
    ] {
        assert!(prepare(&fixture)
            .unwrap()
            .into_capture(&Hasher, before)
            .is_err());
    }
    let marked = prepare(&fixture)
        .unwrap()
        .into_capture(&Hasher, floor)
        .unwrap();
    assert_eq!(marked.recorded_at, floor);
    assert_eq!(marked.review.result.values, corrected.review.result.values);
    measure_administrative_capture_with_history_matches(&Hasher, &marked, &fixture.history)
        .unwrap();
}

#[test]
fn mark_capture_rejects_non_utc_and_unsupported_year_but_accepts_the_exact_prior_clock() {
    let fixture = marking(RecordFixture::initial());
    for invalid in [
        fixture
            .recorded_at
            .to_offset(UtcOffset::from_hms(1, 0, 0).unwrap()),
        Date::from_calendar_date(0, Month::January, 1)
            .unwrap()
            .midnight()
            .assume_utc(),
    ] {
        assert!(prepare(&fixture)
            .unwrap()
            .into_capture(&Hasher, invalid)
            .is_err());
    }
    let prior_at = fixture.history.judicial.groups[0].capture.recorded_at;
    let marked = prepare(&fixture)
        .unwrap()
        .into_capture(&Hasher, prior_at)
        .unwrap();
    assert_eq!(marked.records[0].recorded_at, prior_at);
    measure_administrative_capture_with_history_matches(&Hasher, &marked, &fixture.history)
        .unwrap();
}
