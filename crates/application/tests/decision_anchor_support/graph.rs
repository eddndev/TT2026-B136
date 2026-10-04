use crate::{decision_anchor_support::*, decision_support::*, history_support as hs};
use time::Duration;

fn confirm_fixture(serial: u128, prior: OwnedMeasureMaterial) -> Fixture {
    let mut fixture = fresh_no_change(serial);
    fixture.command.outcome =
        MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
            MeasureEffect::Confirm {
                previous: reference(&prior.capture),
            },
        ]))
        .unwrap();
    fixture.material.result_sources = vec![MeasureResultSources {
        id: prior.capture.result.id,
        sources: prior.capture.result.sources.clone(),
    }];
    fixture.material.predecessors = vec![prior];
    fixture
}

#[test]
fn anchor_targets_and_effect_targets_resolve_separate_exact_root_sets() {
    let first = hs::root_fixture(1, 70).capture();
    let second = hs::root_fixture(2, 80).capture();
    let first_history = hs::history(vec![hs::entry(&first, &empty())]);
    let hearing = review_hearing(
        1,
        &[reference(&first.measures[0])],
        &first_history,
        at() + Duration::seconds(1),
    );
    let mut fixture = confirm_fixture(3, hs::member(&second, 80));
    attach_precautionary(&mut fixture, hearing);
    let evidence = hs::history(vec![
        hs::entry(&first, &empty()),
        hs::entry(&second, &empty()),
    ]);
    let group = capture(fixture.clone(), &evidence, at() + Duration::seconds(2));
    measure_decision_group_with_history_matches(&Hasher, &group, &evidence).unwrap();
    for missing in 0..2 {
        let mut incomplete = evidence.clone();
        incomplete.groups.remove(missing);
        assert!(prepare(fixture.clone(), &incomplete).is_err());
    }
}

#[test]
fn anchor_and_effect_may_select_distinct_revisions_of_one_measure() {
    let first = hs::root_fixture(1, 70).capture();
    let mut evidence = hs::history(vec![hs::entry(&first, &empty())]);
    let hearing = review_hearing(
        1,
        &[reference(&first.measures[0])],
        &evidence,
        at() + Duration::seconds(1),
    );
    let second = hs::confirmation(
        2,
        &[hs::member(&first, 70)],
        &evidence,
        at() + Duration::seconds(2),
    );
    hs::append(&mut evidence, &second);
    let mut fixture = confirm_fixture(3, hs::member(&second, 70));
    attach_precautionary(&mut fixture, hearing);
    let group = capture(fixture, &evidence, at() + Duration::seconds(3));
    assert_eq!(group.measures[0].result.revision.get(), 3);
    measure_decision_group_with_history_matches(&Hasher, &group, &evidence).unwrap();
}

#[test]
fn target_resolution_follows_anchor_dependencies_of_the_owning_group() {
    let first = hs::root_fixture(1, 70).capture();
    let mut evidence = hs::history(vec![hs::entry(&first, &empty())]);
    let hearing = review_hearing(
        1,
        &[reference(&first.measures[0])],
        &evidence,
        at() + Duration::seconds(1),
    );
    let mut fixture = hs::root_fixture(2, 80);
    attach_precautionary(&mut fixture, hearing);
    let anchored = capture(fixture, &evidence, at() + Duration::seconds(2));
    hs::append(&mut evidence, &anchored);
    let selected = [reference(&anchored.measures[0])];
    let targets =
        resolve_measure_targets(&Hasher, anchored.review.case_id, &selected, &evidence).unwrap();
    assert_eq!(targets.targets(), &[hs::member(&anchored, 80)]);
    let mut missing = evidence.clone();
    missing.groups.remove(0);
    assert!(
        resolve_measure_targets(&Hasher, anchored.review.case_id, &selected, &missing).is_err()
    );
}

#[test]
fn anchor_dependency_proof_rejects_wrong_digest_extra_group_and_bad_sibling() {
    let first = Fixture::multiple().capture();
    let evidence = hs::history(vec![hs::entry(&first, &empty())]);
    let hearing = review_hearing(
        1,
        &[reference(&first.measures[0])],
        &evidence,
        at() + Duration::seconds(1),
    );
    let mut fixture = fresh_no_change(3);
    attach_precautionary(&mut fixture, hearing);
    for mutation in 0..3 {
        let mut forged = evidence.clone();
        match mutation {
            0 => {
                forged.groups[0].capture.measures[0].capture_digest =
                    domain::crypto::Sha256Digest::from_array([99; 32])
            }
            1 => {
                let unrelated = hs::root_fixture(9, 90).capture();
                forged.groups.push(hs::entry(&unrelated, &empty()));
            }
            _ => {
                let owner = &mut forged.groups[0].capture;
                owner.measures[1].result.projection.subject.display_name =
                    "Invented sibling".into();
                refresh_digests(owner);
                forged.groups[0] = hs::claimed_entry(owner.clone());
            }
        }
        assert!(prepare(fixture.clone(), &forged).is_err());
    }
}

#[test]
fn immutable_ordinary_hearing_revision_cannot_change_between_valid_groups() {
    let mut first = hs::root_fixture(1, 70);
    attach_initial(&mut first, ordinary_initial());
    let first = capture(first, &empty(), at());
    let mut changed = ordinary_initial();
    changed.snapshot.recorded_by.email = "different retained actor".into();
    let mut second = hs::root_fixture(2, 80);
    attach_initial(&mut second, changed);
    let second = capture(second, &empty(), at());
    let evidence = hs::history(vec![
        hs::entry(&first, &empty()),
        hs::entry(&second, &empty()),
    ]);
    assert!(resolve_measure_targets(
        &Hasher,
        first.review.case_id,
        &[
            reference(&first.measures[0]),
            reference(&second.measures[0])
        ],
        &evidence
    )
    .is_err());
}

#[test]
fn immutable_precautionary_hearing_revision_cannot_change_between_valid_groups() {
    let original = crate::precautionary_receipt_support::scheduled();
    let mut altered = crate::precautionary_receipt_support::Fixture::schedule();
    altered.actor.email = "different retained actor".into();
    let altered = altered.capture(None, at());
    let mut first = hs::root_fixture(1, 70);
    attach_precautionary(&mut first, original);
    let first = capture(first, &empty(), at());
    let mut second = hs::root_fixture(2, 80);
    attach_precautionary(&mut second, altered);
    let second = capture(second, &empty(), at());
    let evidence = hs::history(vec![
        hs::entry(&first, &empty()),
        hs::entry(&second, &empty()),
    ]);
    assert!(resolve_measure_targets(
        &Hasher,
        first.review.case_id,
        &[
            reference(&first.measures[0]),
            reference(&second.measures[0])
        ],
        &evidence
    )
    .is_err());
}

#[test]
fn a_group_cannot_supply_its_own_result_as_an_anchor_target_dependency() {
    let root = hs::root_fixture(1, 70).capture();
    let root_evidence = hs::history(vec![hs::entry(&root, &empty())]);
    let hearing = review_hearing(
        1,
        &[reference(&root.measures[0])],
        &root_evidence,
        at() + Duration::seconds(1),
    );
    let mut forged = root.clone();
    forged.review.command.anchor = Some(MeasureDecisionAnchorRef::Precautionary {
        hearing_id: hearing.review.command.hearing_id,
        revision: hearing.review.result_revision,
        capture_digest: hearing.capture_digest,
    });
    let material = MeasureDecisionAnchorMaterial::Precautionary(Box::new(hearing));
    forged.review.material.anchor = Some(material.clone());
    forged.decision.anchor = Some(material);
    forged.recorded_at = at() + Duration::seconds(2);
    forged.decision.recorded_at = forged.recorded_at;
    for item in &mut forged.measures {
        item.recorded_at = forged.recorded_at;
    }
    refresh_digests(&mut forged);
    for item in &mut forged.measures {
        item.decision_digest = forged.decision.capture_digest;
    }
    refresh_digests(&mut forged);
    let evidence = hs::history(vec![hs::claimed_entry(forged.clone())]);
    assert!(resolve_measure_targets(
        &Hasher,
        forged.review.case_id,
        &[reference(&forged.measures[0])],
        &evidence
    )
    .is_err());
}
