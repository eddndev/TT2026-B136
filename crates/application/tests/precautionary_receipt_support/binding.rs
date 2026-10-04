use application::case_stages::StageDocumentFormat;
use application::participants::participant_digest;
use application::precautionary_hearings::*;
use application::typed_participants::ParticipantCredentialRef;
use application::ApplicationError;
use domain::crypto::{DocumentHasher, Sha256Digest};
use domain::hearings::HearingStatus;
use domain::identity::{Role, UserId};
use domain::participants::{DirectoryStatus, ParticipantRevision, ParticipantValues};
use domain::precautionary_hearings::PrecautionaryHearingRevision;
use time::Duration;
use uuid::Uuid;

use crate::participant_support::{manual_mut, typed_mut};
use crate::precautionary_receipt_support::*;

#[test]
fn source_metadata_and_provenance_change_review_without_changing_the_instruction() {
    let original = scheduled();
    for field in 0..9 {
        let mut fixture = Fixture::schedule();
        match field {
            0 => fixture.sources.support.name = "other-name.pdf".into(),
            1 => fixture.sources.support.format = StageDocumentFormat::Docx,
            2 => {
                manual_mut(&mut fixture.sources.participants[0]).changed_at +=
                    Duration::nanoseconds(1)
            }
            3 => {
                manual_mut(&mut fixture.sources.participants[0])
                    .changed_by
                    .email = "other@example.test".into()
            }
            4 => {
                typed_mut(&mut fixture.sources.participants[1])
                    .changed_by
                    .id = UserId::from_uuid(Uuid::from_u128(99))
            }
            5 => {
                typed_mut(&mut fixture.sources.participants[1]).submission_digest =
                    Sha256Digest::from_array([99; 32])
            }
            6 => {
                let typed = typed_mut(&mut fixture.sources.participants[1]);
                typed.credential_origin = Some(ParticipantCredentialRef {
                    participant_id: typed.id,
                    participant_revision: ParticipantRevision::initial(),
                    statement_digest: Sha256Digest::from_array([7; 32]),
                });
            }
            7 => {
                fixture.sources.participants[1]
                    .bound_subject
                    .as_mut()
                    .unwrap()
                    .changed_by
                    .email = "other-subject@example.test".into()
            }
            _ => {
                let manual = manual_mut(&mut fixture.sources.participants[0]);
                manual.values = ParticipantValues::new(
                    manual.values.display_name(),
                    manual.values.procedural_role(),
                    manual.values.organization(),
                    Some("Different private legal detail"),
                    DirectoryStatus::Active,
                )
                .unwrap();
                manual.values_digest = participant_digest(&Hasher, &manual.values);
            }
        }
        let changed = fixture.capture(None, at());
        assert_eq!(
            changed.review.submission_digest,
            original.review.submission_digest
        );
        assert_ne!(
            precautionary_hearing_review_bytes(&changed.review).unwrap(),
            precautionary_hearing_review_bytes(&original.review).unwrap(),
            "source field {field} is not bound"
        );
        assert_ne!(
            precautionary_hearing_capture_bytes(&changed).unwrap(),
            precautionary_hearing_capture_bytes(&original).unwrap()
        );
    }
}

#[test]
fn cancellation_observed_context_is_bound_separately_from_its_unchanged_instruction() {
    let prior = scheduled();
    let fixture = Fixture::cancel(&prior);
    let first = fixture
        .clone()
        .capture(Some(&prior), at() + Duration::seconds(2));
    let mut changed = fixture;
    let mut material = changed.context.material().clone();
    material.administration.changed_by.email = "other-current-author@example.test".into();
    changed.context = PrecautionaryContext::new(&Hasher, material).unwrap();
    let second = changed.capture(Some(&prior), at() + Duration::seconds(2));
    assert_eq!(first.review.command, second.review.command);
    assert_eq!(
        first.review.submission_digest,
        second.review.submission_digest
    );
    assert_eq!(
        first.review.scheduling_context,
        second.review.scheduling_context
    );
    assert_ne!(
        precautionary_hearing_review_bytes(&first.review).unwrap(),
        precautionary_hearing_review_bytes(&second.review).unwrap()
    );
}

#[test]
fn actual_actor_role_and_capture_time_are_bound_at_their_respective_layers() {
    let litigator = scheduled();
    let mut fixture = Fixture::schedule();
    fixture.actor.role = Role::Owner;
    let owner = fixture.capture(None, at());
    assert_ne!(
        litigator.review.submission_digest,
        owner.review.submission_digest
    );
    assert_ne!(
        precautionary_hearing_review_bytes(&litigator.review).unwrap(),
        precautionary_hearing_review_bytes(&owner.review).unwrap()
    );
    let later = Fixture::schedule().capture(None, at() + Duration::nanoseconds(1));
    assert_eq!(litigator.review, later.review);
    assert_ne!(
        precautionary_hearing_capture_bytes(&litigator).unwrap(),
        precautionary_hearing_capture_bytes(&later).unwrap()
    );
}

fn reframe(capture: &mut PrecautionaryHearingCapture) -> Result<(), ApplicationError> {
    let review = &mut capture.review;
    review.submission_digest = Hasher.hash_bytes(&precautionary_hearing_submission_bytes(
        &review.actor,
        review.case_id,
        &review.command,
        &review.resolved_values,
    )?);
    review.review_digest = Hasher.hash_bytes(&precautionary_hearing_review_bytes(review)?);
    capture.capture_digest = Hasher.hash_bytes(&precautionary_hearing_capture_bytes(capture)?);
    Ok(())
}

#[test]
fn rehashed_receipts_still_reject_semantically_contradictory_derived_fields() {
    let original = scheduled();
    for field in 0..8 {
        let mut capture = original.clone();
        match field {
            0 => capture.review.result_revision = PrecautionaryHearingRevision::new(2).unwrap(),
            1 => capture.review.status = HearingStatus::Cancelled,
            2 => capture.review.participants[0].overview.display_name = "Fabricated name".into(),
            3 => {
                capture.review.participants[0].snapshot.values_digest =
                    Sha256Digest::from_array([99; 32])
            }
            4 => capture.review.actor.role = Role::Client,
            5 => capture.review.scheduling_context = later_context(),
            6 => capture.review.sources.support.digest = Sha256Digest::from_array([99; 32]),
            _ => capture.review.participants.swap(0, 1),
        }
        if reframe(&mut capture).is_ok() {
            assert!(
                precautionary_hearing_receipt_matches(&Hasher, &capture).is_err(),
                "contradiction {field} survived recomputed framing"
            );
        }
    }
}

#[test]
fn each_receipt_digest_is_recomputed_from_retained_material() {
    for field in 0..3 {
        let mut capture = scheduled();
        match field {
            0 => capture.review.submission_digest = Sha256Digest::from_array([99; 32]),
            1 => capture.review.review_digest = Sha256Digest::from_array([99; 32]),
            _ => capture.capture_digest = Sha256Digest::from_array([99; 32]),
        }
        assert!(precautionary_hearing_receipt_matches(&Hasher, &capture).is_err());
    }
}

#[test]
fn receipt_encoders_reject_oversized_sources_and_derived_projection_collections() {
    for projection in [false, true] {
        let mut review = scheduled().review;
        if projection {
            review.participants = vec![review.participants[0].clone(); 33];
        } else {
            review.sources.participants = vec![review.sources.participants[0].clone(); 33];
        }
        assert!(precautionary_hearing_review_bytes(&review).is_err());
    }
}
