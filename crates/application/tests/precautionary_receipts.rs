#[path = "precautionary_receipt_support/binding.rs"]
mod binding;
#[allow(dead_code)]
#[path = "precautionary_context_support/mod.rs"]
mod context_support;
#[path = "precautionary_receipt_support/immutable_sources.rs"]
mod immutable_sources;
#[allow(dead_code)]
#[path = "precautionary_participant_support/mod.rs"]
mod participant_support;
mod precautionary_receipt_support;
#[path = "precautionary_receipt_support/source_supports.rs"]
mod source_supports;
#[path = "precautionary_receipt_support/transitions.rs"]
mod transitions;
#[path = "precautionary_receipt_support/validation.rs"]
mod validation;
#[path = "precautionary_receipt_support/vectors.rs"]
mod vectors;

use application::precautionary_hearings::*;
use domain::crypto::DocumentHasher;
use domain::hearings::HearingStatus;
use precautionary_receipt_support::*;
use time::Duration;

#[test]
fn schedule_preserves_the_actual_actor_and_complete_source_projections() {
    let fixture = Fixture::schedule();
    let prepared = fixture.clone().prepare(None).unwrap();
    let review = prepared.review();
    assert_eq!(review.actor, fixture.actor);
    assert_eq!(review.case_id, fixture.case_id);
    assert_eq!(review.command, fixture.command);
    assert_eq!(review.result_revision.get(), 1);
    assert_eq!(review.status, HearingStatus::Scheduled);
    assert_eq!(review.sources, fixture.sources);
    assert_eq!(review.scheduling_context, fixture.context);
    assert_eq!(review.observed_context, fixture.context);
    assert_eq!(review.participants.len(), 2);
    assert_eq!(
        review.participants[0].overview.display_name,
        "Historical manual name"
    );
    assert_eq!(
        review.participants[1].overview.display_name,
        "Historical person"
    );
    let capture = prepared.into_capture(&Hasher, at()).unwrap();
    precautionary_hearing_receipt_matches(&Hasher, &capture).unwrap();
    let review_bytes = precautionary_hearing_review_bytes(&capture.review).unwrap();
    let capture_bytes = precautionary_hearing_capture_bytes(&capture).unwrap();
    assert!(review_bytes.starts_with(b"PHPR1"));
    assert!(capture_bytes.starts_with(b"PHCR1"));
    assert_eq!(
        capture.review.review_digest,
        Hasher.hash_bytes(&review_bytes)
    );
    assert_eq!(capture.capture_digest, Hasher.hash_bytes(&capture_bytes));
}

#[test]
fn replacement_links_its_exact_predecessor_and_captures_new_context_and_values() {
    let prior = scheduled();
    let next = Fixture::replace(&prior).capture(Some(&prior), at() + Duration::seconds(2));
    assert_eq!(next.review.result_revision.get(), 2);
    assert_eq!(next.review.status, HearingStatus::Scheduled);
    assert_eq!(next.review.scheduling_context, next.review.observed_context);
    assert_ne!(
        next.review.scheduling_context,
        prior.review.scheduling_context
    );
    assert_eq!(
        next.review.resolved_values.venue().as_str(),
        "Replacement court"
    );
    precautionary_hearing_transition_matches(&Hasher, &prior, &next).unwrap();
}

#[test]
fn cancellation_preserves_prior_values_context_and_sources_with_a_new_observation() {
    let prior = scheduled();
    let next = Fixture::cancel(&prior).capture(Some(&prior), at() + Duration::seconds(2));
    assert_eq!(next.review.result_revision.get(), 2);
    assert_eq!(next.review.status, HearingStatus::Cancelled);
    assert_eq!(next.review.resolved_values, prior.review.resolved_values);
    assert_eq!(
        next.review.scheduling_context,
        prior.review.scheduling_context
    );
    assert_eq!(next.review.sources, prior.review.sources);
    assert_eq!(next.review.participants, prior.review.participants);
    assert_ne!(next.review.observed_context, prior.review.observed_context);
    precautionary_hearing_receipt_matches(&Hasher, &next).unwrap();
    precautionary_hearing_transition_matches(&Hasher, &prior, &next).unwrap();
}
