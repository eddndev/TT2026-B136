use crate::{record_decision_support::FixtureV2, record_support::*};
use domain::{crypto::Sha256Digest, precautionary_measures::MeasureDecisionOutcomeInput};
use time::Duration;

#[path = "bounds_tests.rs"]
mod bounds;
#[path = "context_tests.rs"]
mod context;
#[path = "ownership_tests.rs"]
mod ownership;
#[path = "reconstruction_tests.rs"]
mod reconstruction;

fn corrected() -> (RecordFixture, MeasureAdministrativeCapture) {
    let fixture = RecordFixture::initial();
    let capture = fixture.capture();
    (fixture, capture)
}

#[test]
fn judicial_decision_requires_the_exact_administrative_predecessor_owner_and_record() {
    let (prior_fixture, prior) = corrected();
    for mutation in 0..4 {
        let mut fixture = FixtureV2::confirm(&prior, &prior_fixture.history);
        let OwnedMeasureRecord::Administrative { owner, capture } =
            &mut fixture.material.predecessors[0]
        else {
            unreachable!()
        };
        match mutation {
            0 => owner.capture_digest = Sha256Digest::from_array([99; 32]),
            1 => capture.capture_digest = Sha256Digest::from_array([99; 32]),
            2 => {
                capture.result.values = prior_fixture.history.judicial.groups[0].capture.measures[0]
                    .result
                    .values
                    .clone()
            }
            _ => capture.actor.email = "contradictory administrative author".into(),
        }
        assert!(fixture.prepare().is_err());
    }
}

fn effects(fixture: &mut FixtureV2, effects: Vec<MeasureEffect>) {
    fixture.command.outcome =
        MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(effects)).unwrap();
}

#[test]
fn judicial_effects_reject_entered_in_error_predecessors_even_with_exact_valid_receipts() {
    let (first, prior) = corrected();
    let mut marking = RecordFixture::next(&prior, &first.history, 1);
    marking.command.action = MeasureAdministrativeAction::MarkEnteredInError;
    let marked = prepare_measure_administrative_record_with_history(
        &Hasher,
        &marking.actor,
        marking.case_id,
        marking.command,
        marking.context,
        &marking.history,
    )
    .unwrap()
    .into_capture(&Hasher, marking.recorded_at)
    .unwrap();
    let selected = record_reference(&marked.records[0]);
    for effect in [
        MeasureEffect::Confirm { previous: selected },
        MeasureEffect::Modify {
            previous: selected,
            values: marked.review.result.values.clone(),
        },
        MeasureEffect::Revoke { previous: selected },
        MeasureEffect::Cease { previous: selected },
    ] {
        let mut fixture = FixtureV2::confirm(&marked, &marking.history);
        effects(&mut fixture, vec![effect]);
        assert!(fixture.prepare().is_err());
    }
}

#[test]
fn corrected_terminal_judicial_records_cannot_receive_further_judicial_effects() {
    let initial = crate::measure_decision_fixtures::Fixture::single().capture();
    for cease in [false, true] {
        let mut terminal = crate::effect_support::LaterFixture::confirm(&initial);
        let selected = reference(&initial.measures[0]);
        terminal.effects(vec![if cease {
            MeasureEffect::Cease { previous: selected }
        } else {
            MeasureEffect::Revoke { previous: selected }
        }]);
        let ancestors = terminal.evidence.clone();
        let group = terminal.capture();
        let correction =
            RecordFixture::from_first(CorrectionFixture::from_group(&group, &ancestors, id(70)));
        let capture = correction.capture();
        assert_eq!(
            capture.review.result.validity,
            MeasureCaptureValidity::Valid
        );
        assert!(FixtureV2::confirm(&capture, &correction.history)
            .prepare()
            .is_err());
    }
}

#[test]
fn retained_effects_cannot_rewrite_administrative_source_provenance() {
    let (prior_fixture, prior) = corrected();
    let selected = record_reference(&prior.records[0]);
    for effect in [
        MeasureEffect::Confirm { previous: selected },
        MeasureEffect::Revoke { previous: selected },
        MeasureEffect::Cease { previous: selected },
    ] {
        let mut fixture = FixtureV2::confirm(&prior, &prior_fixture.history);
        effects(&mut fixture, vec![effect]);
        fixture.material.result_sources[0]
            .sources
            .subject
            .changed_by
            .email = "rewritten retained source".into();
        assert!(fixture.prepare().is_err());
    }
}

#[test]
fn v2_capture_floor_includes_selected_administrative_time() {
    let (prior_fixture, prior) = corrected();
    let fixture = FixtureV2::confirm(&prior, &prior_fixture.history);
    let before = prior.recorded_at - Duration::nanoseconds(1);
    assert!(before > prior_fixture.history.judicial.groups[0].capture.recorded_at);
    assert!(fixture
        .clone()
        .prepare()
        .unwrap()
        .into_group_capture(&Hasher, before)
        .is_err());
    fixture
        .prepare()
        .unwrap()
        .into_group_capture(&Hasher, prior.recorded_at)
        .unwrap();
}
