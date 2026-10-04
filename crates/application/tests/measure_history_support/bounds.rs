use crate::{decision_support::*, measure_history_support::*};
use time::Duration;

#[test]
fn target_selection_rejects_duplicate_identities_even_at_different_revisions() {
    let (group, evidence) = three_revisions();
    let selected = reference(&group.measures[0]);
    let initial = reference(&evidence.groups[0].capture.measures[0]);
    for duplicate in [selected, initial] {
        assert!(resolve_measure_targets(
            &Hasher,
            group.review.case_id,
            &[selected, duplicate],
            &evidence
        )
        .is_err());
    }
}

#[test]
fn target_selection_accepts_32_distinct_identities_and_rejects_33() {
    let mut fixture = root_fixture(1, 70);
    let sources = fixture.material.result_sources[0].sources.clone();
    let values = MeasureValues::new(crate::measure_source_support::input(&sources));
    for measure in 71..102 {
        fixture.add_imposition(measure, values.clone(), sources.clone());
    }
    let first = fixture.capture();
    let mut evidence = history(vec![entry(&first, &empty())]);
    let mut selected = first.measures.iter().map(reference).collect::<Vec<_>>();
    assert_eq!(
        resolve_measure_targets(&Hasher, first.review.case_id, &selected, &evidence)
            .unwrap()
            .targets()
            .len(),
        32
    );
    let second = root_fixture(2, 102).capture();
    evidence.groups.push(entry(&second, &empty()));
    selected.push(reference(&second.measures[0]));
    assert!(resolve_measure_targets(&Hasher, first.review.case_id, &selected, &evidence).is_err());
}

#[test]
fn history_accepts_256_groups_and_rejects_257_before_resolution() {
    let mut latest = root_fixture(1, 70).capture();
    let mut evidence = history(vec![entry(&latest, &empty())]);
    for serial in 2..=256 {
        let next = confirmation(
            serial,
            &[member(&latest, 70)],
            &evidence,
            at() + Duration::seconds(serial as i64),
        );
        append(&mut evidence, &next);
        latest = next;
    }
    assert_eq!(evidence.groups.len(), 256);
    let selected = [reference(&latest.measures[0])];
    assert!(resolve_measure_targets(&Hasher, latest.review.case_id, &selected, &evidence).is_ok());
    let ancestors = history(evidence.groups[..255].to_vec());
    assert!(measure_group_origin(&Hasher, &latest, &ancestors).is_ok());
    let mut next = root_fixture(257, 999);
    next.command.outcome = MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
        MeasureEffect::Confirm {
            previous: selected[0],
        },
    ]))
    .unwrap();
    next.material.predecessors = vec![member(&latest, 70)];
    next.material.result_sources = vec![MeasureResultSources {
        id: id(70),
        sources: latest.measures[0].result.sources.clone(),
    }];
    assert!(prepare_measure_decision_with_history(
        &Hasher,
        &next.actor,
        next.case_id,
        next.command,
        next.material,
        &evidence,
    )
    .is_err());
    let extra = root_fixture(999, 90).capture();
    evidence.groups.push(entry(&extra, &empty()));
    assert!(resolve_measure_targets(&Hasher, latest.review.case_id, &selected, &evidence).is_err());
}

#[test]
fn oversized_member_evidence_is_rejected_without_accepting_a_partial_inventory() {
    let group = root_fixture(1, 70).capture();
    let mut oversized = entry(&group, &empty());
    oversized.capture.measures = vec![group.measures[0].clone(); 8193];
    rejects(&group, &history(vec![oversized]));
}
