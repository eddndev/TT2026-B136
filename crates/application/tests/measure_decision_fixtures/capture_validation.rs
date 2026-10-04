use super::*;
use application::typed_participants::ParticipantRevisionSnapshot;
use domain::cases::CaseId;
use domain::crypto::Sha256Digest;
use domain::hearings::HearingNote;
use domain::identity::{Role, UserId};
use time::{Date, Duration, Month, UtcOffset};
use uuid::Uuid;

#[test]
fn every_nested_commitment_and_the_causing_decision_digest_is_verified() {
    for mutation in 0..6 {
        let mut group = Fixture::single().capture();
        let digest = Sha256Digest::from_array([99; 32]);
        match mutation {
            0 => group.review.submission_digest = digest,
            1 => group.review.review_digest = digest,
            2 => group.decision.capture_digest = digest,
            3 => group.measures[0].capture_digest = digest,
            4 => group.measures[0].decision_digest = digest,
            _ => group.capture_digest = digest,
        }
        assert_invalid(&group);
    }
}

#[test]
fn rehashed_result_copies_must_equal_the_values_derived_from_the_command_and_sources() {
    for in_review in [false, true] {
        for mutation in 0..13 {
            let mut group = Fixture::single().capture();
            let prior = reference(&group.measures[0]);
            let result = if in_review {
                &mut group.review.results[0]
            } else {
                &mut group.measures[0].result
            };
            match mutation {
                0 => result.id = id(99),
                1 => result.revision = MeasureRevision::new(2).unwrap(),
                2 => {
                    result.origin.operation_id =
                        MeasureDecisionOperationId::from_uuid(Uuid::from_u128(101))
                }
                3 => result.origin.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(111)),
                4 => result.effect_key = id(99),
                5 => result.action = MeasureCaptureAction::Confirm,
                6 => result.previous = Some(prior),
                7 => {
                    let mut values = crate::measure_source_support::input(&result.sources);
                    values.conditions = HearingNote::new("Different conditions").unwrap();
                    result.values = MeasureValues::new(values);
                }
                8 => result.sources.subject.changed_by.email = "different@example.test".into(),
                9 => result.projection.subject.display_name = "Invented label".into(),
                10 => {
                    result
                        .projection
                        .supervisor
                        .as_mut()
                        .unwrap()
                        .overview
                        .display_name = "Invented supervisor".into()
                }
                11 => {
                    result
                        .projection
                        .supervisor
                        .as_mut()
                        .unwrap()
                        .snapshot
                        .values_digest = Sha256Digest::from_array([99; 32])
                }
                _ => {
                    let ParticipantRevisionSnapshot::Typed(snapshot) =
                        &mut result.sources.supervisor.as_mut().unwrap().revision
                    else {
                        panic!("typed supervisor fixture expected")
                    };
                    snapshot.submission_digest = Sha256Digest::from_array([99; 32]);
                }
            }
            refresh_digests(&mut group);
            assert_invalid(&group);
        }
    }
}

#[test]
fn equally_forged_review_and_measure_origins_do_not_override_the_actual_creation_ids() {
    for mutation in 0..2 {
        let mut group = Fixture::single().capture();
        for result in [&mut group.review.results[0], &mut group.measures[0].result] {
            if mutation == 0 {
                result.origin.operation_id =
                    MeasureDecisionOperationId::from_uuid(Uuid::from_u128(101));
            } else {
                result.origin.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(111));
            }
        }
        refresh_digests(&mut group);
        assert_invalid(&group);
    }
}

#[test]
fn rehashed_decision_fields_must_agree_with_the_checked_review() {
    for mutation in 0..9 {
        let mut group = Fixture::single().capture();
        let decision = &mut group.decision;
        match mutation {
            0 => decision.case_id = CaseId::from_uuid(Uuid::from_u128(99)),
            1 => {
                decision.operation_id = MeasureDecisionOperationId::from_uuid(Uuid::from_u128(101))
            }
            2 => decision.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(111)),
            3 => decision.actor.email = "different@example.test".into(),
            4 => decision.context = crate::precautionary_receipt_support::later_context(),
            5 => {
                let mut values = decision_input(&decision.values);
                values.authority = HearingNote::new("Different authority").unwrap();
                decision.values = MeasureDecisionValues::new(values);
            }
            6 => decision.support.name = "different.pdf".into(),
            7 => decision.support.digest = Sha256Digest::from_array([99; 32]),
            _ => decision.recorded_at += Duration::seconds(1),
        }
        refresh_digests(&mut group);
        for member in &mut group.measures {
            member.decision_digest = group.decision.capture_digest;
        }
        refresh_digests(&mut group);
        assert_invalid(&group);
    }
}

#[test]
fn rehashed_measure_cause_actor_and_clock_must_match_the_owning_group() {
    for mutation in 0..8 {
        let mut group = Fixture::single().capture();
        let measure = &mut group.measures[0];
        match mutation {
            0 => measure.case_id = CaseId::from_uuid(Uuid::from_u128(99)),
            1 => measure.operation_id = MeasureDecisionOperationId::from_uuid(Uuid::from_u128(101)),
            2 => measure.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(111)),
            3 => measure.decision_digest = Sha256Digest::from_array([99; 32]),
            4 => measure.actor.id = UserId::from_uuid(Uuid::from_u128(3)),
            5 => measure.actor.email = "different@example.test".into(),
            6 => measure.actor.role = Role::Owner,
            _ => measure.recorded_at += Duration::seconds(1),
        }
        refresh_digests(&mut group);
        assert_invalid(&group);
    }
}

#[test]
fn group_members_and_review_results_are_exact_complete_ordered_sets() {
    for in_review in [false, true] {
        for mutation in 0..4 {
            let mut group = Fixture::multiple().capture();
            if in_review {
                let results = &mut group.review.results;
                match mutation {
                    0 => {
                        results.pop();
                    }
                    1 => results.push(results[0].clone()),
                    2 => results.swap(0, 1),
                    _ => results[1] = results[0].clone(),
                }
            } else {
                let members = &mut group.measures;
                match mutation {
                    0 => {
                        members.pop();
                    }
                    1 => members.push(members[0].clone()),
                    2 => members.swap(0, 1),
                    _ => members[1] = members[0].clone(),
                }
            }
            refresh_digests(&mut group);
            assert_invalid(&group);
        }
    }
}

#[test]
fn no_change_groups_cannot_smuggle_an_imposed_member_or_result() {
    let source = Fixture::single().capture();
    for in_review in [false, true] {
        let mut group = Fixture::no_change().capture();
        if in_review {
            group.review.results = source.review.results.clone();
        } else {
            group.measures = source.measures.clone();
        }
        refresh_digests(&mut group);
        assert_invalid(&group);
    }
}

#[test]
fn standalone_groups_cannot_append_invented_substitution_relationships() {
    let mut group = Fixture::single().capture();
    let result = reference(&group.measures[0]);
    group.substitutions.push(MeasureSubstitutionCapture {
        effect_key: id(70),
        predecessors: vec![MeasureSubstitutionPredecessor {
            previous: result,
            result,
        }],
        successors: vec![result],
    });
    refresh_group_digest(&mut group);
    assert_invalid(&group);
}

#[test]
fn public_group_timestamp_must_be_supported_utc_and_equal_every_member_timestamp() {
    for timestamp in [
        at() + Duration::seconds(1),
        at().to_offset(UtcOffset::from_hms(1, 0, 0).unwrap()),
        Date::from_calendar_date(0, Month::January, 1)
            .unwrap()
            .midnight()
            .assume_utc(),
    ] {
        let mut group = Fixture::single().capture();
        group.recorded_at = timestamp;
        assert_invalid(&group);
    }
    let mut group = Fixture::single().capture();
    group.recorded_at += Duration::seconds(1);
    refresh_group_digest(&mut group);
    assert_invalid(&group);
}

#[test]
fn rewriting_all_capture_times_before_source_history_remains_invalid_after_rehashing() {
    let mut group = Fixture::single().capture();
    let too_early = crate::context_support::at() - Duration::nanoseconds(1);
    group.recorded_at = too_early;
    group.decision.recorded_at = too_early;
    for measure in &mut group.measures {
        measure.recorded_at = too_early;
    }
    refresh_digests(&mut group);
    for measure in &mut group.measures {
        measure.decision_digest = group.decision.capture_digest;
    }
    refresh_digests(&mut group);
    assert_invalid(&group);
}

#[test]
fn historical_recording_role_cannot_be_rewritten_as_a_read_only_role() {
    for role in [Role::Paralegal, Role::Client] {
        let mut group = Fixture::single().capture();
        group.review.actor.role = role;
        group.decision.actor.role = role;
        for measure in &mut group.measures {
            measure.actor.role = role;
        }
        refresh_digests(&mut group);
        for measure in &mut group.measures {
            measure.decision_digest = group.decision.capture_digest;
        }
        refresh_digests(&mut group);
        assert_invalid(&group);
    }
}
