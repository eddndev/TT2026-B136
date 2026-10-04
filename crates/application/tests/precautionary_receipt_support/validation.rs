use application::cases::{case_administration_digest, CaseAdministrativeStatus, CaseRevision};
use application::precautionary_hearings::*;
use domain::case_administration::CaseStageRevision;
use domain::cases::CaseId;
use domain::crypto::{DocumentId, DocumentVersion, Sha256Digest};
use domain::identity::Role;
use domain::participants::ParticipantRevision;
use domain::precautionary_hearings::*;
use time::{Date, Duration, Month, UtcOffset};
use uuid::Uuid;

use crate::participant_support::{manual_mut, typed_mut};
use crate::precautionary_receipt_support::*;

#[test]
fn captures_retain_authorized_historical_roles_and_reject_other_recording_roles() {
    for role in [Role::Owner, Role::Litigator, Role::Paralegal, Role::Client] {
        let mut fixture = Fixture::schedule();
        fixture.actor.role = role;
        match role {
            Role::Owner | Role::Litigator => {
                let capture = fixture.capture(None, at());
                assert_eq!(capture.review.actor.role, role);
                precautionary_hearing_receipt_matches(&Hasher, &capture).unwrap();
            }
            Role::Paralegal | Role::Client => assert!(fixture.prepare(None).is_err()),
        }
    }
}

#[test]
fn captured_actor_text_is_checked_without_inventing_an_email_address_grammar() {
    for email in ["", " actor", "actor ", "actor\nlabel", "actor\u{7f}"] {
        let mut fixture = Fixture::schedule();
        fixture.actor.email = email.into();
        assert!(fixture.prepare(None).is_err());
    }
    let mut fixture = Fixture::schedule();
    fixture.actor.email = "historical account label".into();
    assert!(fixture.prepare(None).is_ok());
}

#[test]
fn review_purpose_is_rejected_without_a_real_measure_capture() {
    let mut fixture = Fixture::schedule();
    let mut input = values_input();
    input.purpose = PrecautionaryHearingPurpose::Review;
    input.review_targets.push(PrecautionaryMeasureRef::new(
        MeasureId::from_uuid(Uuid::from_u128(80)),
        MeasureRevision::initial(),
        Sha256Digest::from_array([8; 32]),
    ));
    fixture.command.change = PrecautionaryHearingChange::Schedule {
        context: expectation(&fixture.context),
        values: PrecautionaryHearingValues::new(input).unwrap(),
    };
    assert!(fixture.prepare(None).is_err());
}

#[test]
fn scheduling_requires_the_exact_case_and_reviewed_context_expectation() {
    for field in 0..4 {
        let mut fixture = Fixture::schedule();
        if field == 0 {
            fixture.case_id = CaseId::from_uuid(Uuid::from_u128(99));
        } else {
            let PrecautionaryHearingChange::Schedule { context, .. } = &mut fixture.command.change
            else {
                unreachable!()
            };
            match field {
                1 => context.administration_revision = CaseRevision::new(2).unwrap(),
                2 => context.stage_revision = CaseStageRevision::new(2).unwrap(),
                _ => context.context_digest = Sha256Digest::from_array([99; 32]),
            }
        }
        assert!(fixture.prepare(None).is_err());
    }
}

#[test]
fn historical_closed_context_is_not_valid_for_a_new_appointment_mutation() {
    let prior = scheduled();
    for mut fixture in [
        Fixture::schedule(),
        Fixture::replace(&prior),
        Fixture::cancel(&prior),
    ] {
        let mut material = later_context().material().clone();
        material.administration.values = material
            .administration
            .values
            .with_status(CaseAdministrativeStatus::Closed);
        material.administration.values_digest =
            case_administration_digest(&Hasher, &material.administration.values);
        fixture.context = PrecautionaryContext::new(&Hasher, material).unwrap();
        match &mut fixture.command.change {
            PrecautionaryHearingChange::Schedule { context, .. }
            | PrecautionaryHearingChange::Replace { context, .. } => {
                *context = expectation(&fixture.context)
            }
            PrecautionaryHearingChange::Cancel { .. } => {}
        }
        let base = (fixture.command.expected_revision() != 0).then_some(&prior);
        assert!(fixture.prepare(base).is_err());
    }
}

#[test]
fn source_participants_require_exact_count_scope_revisions_and_bound_subjects() {
    for mutation in 0..7 {
        let mut fixture = Fixture::schedule();
        match mutation {
            0 => fixture.sources.participants.clear(),
            1 => {
                fixture
                    .sources
                    .participants
                    .push(fixture.sources.participants[0].clone());
            }
            2 => fixture.sources.participants[1] = fixture.sources.participants[0].clone(),
            3 => {
                manual_mut(&mut fixture.sources.participants[0]).case_id =
                    CaseId::from_uuid(Uuid::from_u128(99))
            }
            4 => {
                manual_mut(&mut fixture.sources.participants[0]).revision =
                    ParticipantRevision::new(2).unwrap()
            }
            5 => {
                manual_mut(&mut fixture.sources.participants[0]).values_digest =
                    Sha256Digest::from_array([99; 32])
            }
            _ => fixture.sources.participants[1].bound_subject = None,
        }
        assert!(fixture.prepare(None).is_err());
    }
}

#[test]
fn scheduling_support_requires_exact_content_and_a_safe_retained_name() {
    for mutation in 0..4 {
        let mut fixture = Fixture::schedule();
        match mutation {
            0 => fixture.sources.support.reference.id = DocumentId::from_uuid(Uuid::from_u128(99)),
            1 => fixture.sources.support.reference.version = DocumentVersion::new(2).unwrap(),
            2 => fixture.sources.support.digest = Sha256Digest::from_array([99; 32]),
            _ => fixture.sources.support.name = "../escape.pdf".into(),
        }
        assert!(fixture.prepare(None).is_err());
    }
}

#[test]
fn participant_and_subject_provenance_requires_supported_utc_and_clean_actor_text() {
    for location in 0..3 {
        for invalid_time in [false, true] {
            let mut fixture = Fixture::schedule();
            let (timestamp, author) = match location {
                0 => {
                    let source = manual_mut(&mut fixture.sources.participants[0]);
                    (&mut source.changed_at, &mut source.changed_by)
                }
                1 => {
                    let source = typed_mut(&mut fixture.sources.participants[1]);
                    (&mut source.changed_at, &mut source.changed_by)
                }
                _ => {
                    let source = fixture.sources.participants[1]
                        .bound_subject
                        .as_mut()
                        .unwrap();
                    (&mut source.changed_at, &mut source.changed_by)
                }
            };
            if invalid_time {
                *timestamp = timestamp.to_offset(UtcOffset::from_hms(1, 0, 0).unwrap());
            } else {
                author.email = "untrimmed actor ".into();
            }
            assert!(fixture.prepare(None).is_err());
        }
    }
}

#[test]
fn capture_clock_requires_supported_utc_and_cannot_predate_any_retained_source() {
    for timestamp in [
        at().to_offset(UtcOffset::from_hms(1, 0, 0).unwrap()),
        Date::from_calendar_date(0, Month::January, 1)
            .unwrap()
            .midnight()
            .assume_utc(),
        context().material().administration.changed_at - Duration::nanoseconds(1),
    ] {
        assert!(Fixture::schedule()
            .prepare(None)
            .unwrap()
            .into_capture(&Hasher, timestamp)
            .is_err());
    }
    let mut fixture = Fixture::schedule();
    manual_mut(&mut fixture.sources.participants[0]).changed_at = at() + Duration::seconds(1);
    assert!(fixture
        .prepare(None)
        .unwrap()
        .into_capture(&Hasher, at())
        .is_err());
    let prior = scheduled();
    assert!(Fixture::cancel(&prior)
        .prepare(Some(&prior))
        .unwrap()
        .into_capture(&Hasher, at())
        .is_err());
    let prior = Fixture::schedule().capture(None, at() + Duration::seconds(10));
    assert!(Fixture::replace(&prior)
        .prepare(Some(&prior))
        .unwrap()
        .into_capture(&Hasher, at() + Duration::seconds(5))
        .is_err());
}
