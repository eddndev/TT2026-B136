use super::*;
use domain::crypto::DocumentHasher;
use uuid::Uuid;

pub(super) fn refresh(group: &mut MeasureDecisionGroupCaptureV2) {
    let review = &mut group.review;
    review.submission_digest = Hasher.hash_bytes(
        &measure_decision_submission_bytes(&review.actor, review.case_id, &review.command).unwrap(),
    );
    review.review_digest = Hasher.hash_bytes(&measure_decision_review_v2_bytes(review).unwrap());
    group.decision.capture_digest =
        Hasher.hash_bytes(&measure_decision_capture_bytes(&group.decision).unwrap());
    for row in &mut group.measures {
        row.decision_digest = group.decision.capture_digest;
        row.capture_digest = Hasher.hash_bytes(&measure_capture_v2_bytes(row).unwrap());
    }
    group.capture_digest = Hasher.hash_bytes(&measure_decision_group_v2_bytes(group).unwrap());
}

pub(super) fn claimed_entry(group: MeasureDecisionGroupCaptureV2) -> MeasureGroupEvidenceV2 {
    MeasureGroupEvidenceV2 {
        origin: MeasureGroupOrigin {
            case_id: group.review.case_id,
            operation_id: group.review.command.operation_id,
            decision_id: group.review.command.decision_id,
            submission_digest: group.review.submission_digest,
            review_digest: group.review.review_digest,
            decision_digest: group.decision.capture_digest,
            group_digest: group.capture_digest,
        },
        capture: group,
    }
}

fn rejected(group: &MeasureDecisionGroupCaptureV2, fixture: &FixtureV2) {
    assert!(measure_decision_group_v2_matches(&Hasher, group, &fixture.history).is_err());
    assert!(measure_group_origin_v2(&Hasher, group, &fixture.history).is_err());
    let mut evidence = fixture.history.clone();
    evidence.decisions.push(claimed_entry(group.clone()));
    let selected = crate::record_decision_support::reference_v2(&group.measures[0]);
    assert!(resolve_measure_records_with_decision_history(
        &Hasher,
        fixture.case_id,
        &[selected],
        &evidence
    )
    .is_err());
}

#[test]
fn rehashed_retained_judicial_results_cannot_resurrect_values_from_before_correction() {
    let (prior_fixture, prior) = corrected();
    let old_values = prior_fixture.history.judicial.groups[0].capture.measures[0]
        .result
        .values
        .clone();
    assert_ne!(old_values, prior.review.result.values);
    let selected = record_reference(&prior.records[0]);
    for effect in [
        MeasureEffect::Confirm { previous: selected },
        MeasureEffect::Revoke { previous: selected },
        MeasureEffect::Cease { previous: selected },
    ] {
        let mut fixture = FixtureV2::confirm(&prior, &prior_fixture.history);
        effects(&mut fixture, vec![effect]);
        let mut group = fixture.capture();
        group.review.results[0].values = old_values.clone();
        group.measures[0].result = group.review.results[0].clone();
        refresh(&mut group);
        rejected(&group, &fixture);
    }
}

#[test]
fn v2_reconstruction_binds_record_root_judicial_origin_effect_and_corrected_previous() {
    let (prior_fixture, prior) = corrected();
    let fixture = FixtureV2::confirm(&prior, &prior_fixture.history);
    let original = fixture.capture();
    for mutation in 0..9 {
        let mut group = original.clone();
        let result = &mut group.review.results[0];
        match mutation {
            0 => {
                result.record_root = MeasureRecordRoot::Administrative {
                    operation_id: prior.review.command.operation_id,
                    measure_id: result.id,
                }
            }
            1 => {
                result.judicial_origin = MeasureOriginIds {
                    decision_id: group.review.command.decision_id,
                    operation_id: group.review.command.operation_id,
                }
            }
            2 => result.previous = Some(prior.review.result.last_judicial.reference),
            3 => result.revision = MeasureRevision::new(9).unwrap(),
            4 => result.action = MeasureCaptureAction::Modify,
            5 => result.sources.subject.changed_by.email = "contradictory retained actor".into(),
            6 => result.projection.subject.display_name = "Invented effective subject".into(),
            7 => result.effect_key = id(999),
            _ => {
                result.record_root = MeasureRecordRoot::Judicial(MeasureOriginIds {
                    decision_id: result.judicial_origin.decision_id,
                    operation_id: MeasureDecisionOperationId::from_uuid(Uuid::from_u128(999)),
                })
            }
        }
        group.measures[0].result = group.review.results[0].clone();
        refresh(&mut group);
        rejected(&group, &fixture);
    }
}

#[test]
fn v2_group_must_own_its_exact_complete_row_set_after_rehashing() {
    let fixture = FixtureV2::initial(crate::measure_decision_fixtures::Fixture::multiple());
    let original = fixture.capture();
    for mutation in 0..4 {
        let mut group = original.clone();
        match mutation {
            0 => {
                group.measures.pop();
            }
            1 => {
                group.measures[1].result.projection.subject.display_name = "Invented sibling".into()
            }
            2 => {
                let mut row = group.measures[1].clone();
                row.result.id = id(999);
                group.measures.push(row);
            }
            _ => group.measures[1].result.id = group.measures[0].result.id,
        }
        refresh(&mut group);
        rejected(&group, &fixture);
    }
}

#[test]
fn coherently_rehashed_v2_capture_cannot_predate_its_effective_administrative_predecessor() {
    let (prior_fixture, prior) = corrected();
    let fixture = FixtureV2::confirm(&prior, &prior_fixture.history);
    let mut group = fixture.capture();
    let at = prior.recorded_at - Duration::nanoseconds(1);
    group.recorded_at = at;
    group.decision.recorded_at = at;
    for row in &mut group.measures {
        row.recorded_at = at;
    }
    refresh(&mut group);
    rejected(&group, &fixture);
}
