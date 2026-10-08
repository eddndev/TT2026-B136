use application::participants::participant_digest;
use application::precautionary_hearings::*;
use domain::crypto::Sha256Digest;
use domain::hearings::HearingVenue;
use domain::participants::{DirectoryStatus, ParticipantValues};
use domain::precautionary_hearings::PrecautionaryHearingValues;
use time::Duration;

use crate::participant_support::{manual_mut, typed, typed_mut};
use crate::precautionary_receipt_support::*;

fn assert_locally_consistent_sources(fixture: &Fixture) {
    let mut standalone = fixture.clone();
    let PrecautionaryHearingChange::Replace { values, .. } = &fixture.command.change else {
        panic!("replacement fixture expected")
    };
    standalone.command.change = PrecautionaryHearingChange::Schedule {
        context: expectation(&standalone.context),
        values: values.clone(),
    };
    assert!(standalone.prepare(None).is_ok());
}

#[test]
fn regression_replacement_preserves_full_manual_material_for_the_same_revision() {
    let prior = scheduled();
    for changed_values in [false, true] {
        let mut fixture = Fixture::replace(&prior);
        let source = manual_mut(&mut fixture.sources.participants[0]);
        if changed_values {
            source.values = ParticipantValues::new(
                source.values.display_name(),
                source.values.procedural_role(),
                source.values.organization(),
                Some("Changed private legal text at the same revision"),
                source.values.directory_status(),
            )
            .unwrap();
            source.values_digest = participant_digest(&Hasher, &source.values);
        } else {
            source.changed_by.email = "contradictory-author@example.test".into();
        }
        assert_locally_consistent_sources(&fixture);
        assert!(fixture.prepare(Some(&prior)).is_err());
    }
}

#[test]
fn regression_replacement_preserves_full_typed_material_for_the_same_revision() {
    let prior = scheduled();
    for field in 0..3 {
        let mut fixture = Fixture::replace(&prior);
        let source = typed_mut(&mut fixture.sources.participants[1]);
        match field {
            0 => source.changed_by.email = "contradictory-author@example.test".into(),
            1 => source.changed_at += Duration::seconds(1),
            _ => source.submission_digest = Sha256Digest::from_array([99; 32]),
        }
        assert_locally_consistent_sources(&fixture);
        assert!(fixture.prepare(Some(&prior)).is_err());
    }
}

fn replacement_with_new_participant_revision(prior: &PrecautionaryHearingCapture) -> Fixture {
    let mut fixture = Fixture::replace(prior);
    let mut input = crate::participant_support::hearing_input(&[(10, 1), (20, 3)]);
    input.scheduled_at = values_input().scheduled_at;
    input.venue = HearingVenue::new("Replacement court").unwrap();
    let PrecautionaryHearingChange::Replace { values, .. } = &mut fixture.command.change else {
        unreachable!()
    };
    *values = PrecautionaryHearingValues::new(input).unwrap();
    fixture.sources.participants[1] = typed(20, 3, false, DirectoryStatus::Active);
    typed_mut(&mut fixture.sources.participants[1]).changed_at += Duration::seconds(1);
    fixture
}

#[test]
fn regression_new_participant_revisions_retain_the_exact_reused_subject_snapshot() {
    let prior = scheduled();
    let valid = replacement_with_new_participant_revision(&prior);
    assert_locally_consistent_sources(&valid);
    let next = valid
        .clone()
        .capture(Some(&prior), at() + Duration::seconds(2));
    assert_ne!(
        next.review.sources.participants[1].revision_number(),
        prior.review.sources.participants[1].revision_number()
    );
    assert_eq!(
        next.review.sources.participants[1].bound_subject,
        prior.review.sources.participants[1].bound_subject
    );
    precautionary_hearing_transition_matches(&Hasher, &prior, &next).unwrap();
    for changed_time in [false, true] {
        let mut fixture = valid.clone();
        let subject = fixture.sources.participants[1]
            .bound_subject
            .as_mut()
            .unwrap();
        if changed_time {
            subject.changed_at += Duration::nanoseconds(1);
        } else {
            subject.changed_by.email = "contradictory-subject-author@example.test".into();
        }
        assert_eq!(
            subject.values_digest,
            prior.review.sources.participants[1]
                .bound_subject
                .as_ref()
                .unwrap()
                .values_digest
        );
        assert_locally_consistent_sources(&fixture);
        assert!(fixture.prepare(Some(&prior)).is_err());
    }
}
