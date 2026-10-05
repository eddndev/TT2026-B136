use super::*;
use domain::DomainError;
use std::{
    io::Read,
    sync::atomic::{AtomicUsize, Ordering},
};

fn reject(fixture: Fixture) {
    let harness = harness(fixture.store(), identity(fixture.actor));
    assert!(harness
        .service
        .prepare("session", fixture.case_id, fixture.command)
        .is_err());
}

#[test]
fn ready_requires_complete_exact_owned_predecessors_sources_scope_and_mixed_closure() {
    for mutation in 0..9 {
        let mut fixture = Fixture::corrected();
        match mutation {
            0 => fixture
                .material
                .record_history
                .records
                .judicial
                .groups
                .clear(),
            1 => fixture
                .material
                .record_history
                .records
                .administrative
                .clear(),
            2 => fixture
                .material
                .record_history
                .records
                .administrative
                .push(fixture.material.record_history.records.administrative[0].clone()),
            3 => fixture.material.predecessors.clear(),
            4 => {
                let OwnedMeasureRecord::Administrative { capture, .. } =
                    &mut fixture.material.predecessors[0]
                else {
                    unreachable!()
                };
                capture.actor.email = "different-captured-actor@example.test".into();
            }
            5 => fixture.material.result_sources.clear(),
            6 => {
                fixture.material.result_sources[0]
                    .sources
                    .subject
                    .changed_by
                    .email = "contradictory-retained-source@example.test".into()
            }
            7 => fixture.case_id = CaseId::new(),
            _ => fixture.command.context.context_digest = Sha256Digest::from_array([66; 32]),
        }
        reject(fixture);
    }
}

#[test]
fn a_real_m2_selection_requires_its_full_g2_owner_and_original_g1_a_ancestry() {
    let prior = Fixture::corrected().operation(now() - Duration::seconds(1));
    for mutation in 0..4 {
        let mut fixture = next(&prior);
        match mutation {
            0 => fixture.material.record_history.decisions.clear(),
            1 => fixture
                .material
                .record_history
                .records
                .judicial
                .groups
                .clear(),
            2 => fixture.material.record_history.decisions[0]
                .capture
                .measures
                .clear(),
            _ => {
                fixture.material.record_history.decisions[0]
                    .origin
                    .group_digest = Sha256Digest::from_array([77; 32])
            }
        }
        reject(fixture);
    }
}

#[test]
fn terminal_or_entered_in_error_records_cannot_receive_fresh_judicial_effects() {
    let mut terminal = Fixture::corrected();
    let previous = terminal.command.outcome.changes().unwrap()[0].clone();
    let MeasureEffect::Confirm { previous } = previous else {
        unreachable!()
    };
    terminal.command.outcome =
        MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
            MeasureEffect::Revoke { previous },
        ]))
        .unwrap();
    let revoked = terminal.operation(now() - Duration::seconds(1));
    reject(next(&revoked));

    let mut correction = crate::record_support::RecordFixture::initial();
    correction.command.action =
        application::measure_corrections::MeasureAdministrativeAction::MarkEnteredInError;
    let marked =
        application::measure_corrections::prepare_measure_administrative_record_with_history(
            &Hasher,
            &correction.actor,
            correction.case_id,
            correction.command.clone(),
            correction.context.clone(),
            &correction.history,
        )
        .unwrap()
        .into_capture(&Hasher, correction.recorded_at)
        .unwrap();
    assert_eq!(
        marked.review.result.validity,
        MeasureCaptureValidity::EnteredInError
    );
    reject(from_pure(
        crate::record_decision_support::FixtureV2::confirm(&marked, &correction.history),
        97,
    ));
}

#[derive(Default)]
struct CountHasher(AtomicUsize);
impl DocumentHasher for CountHasher {
    fn hash_bytes(&self, value: &[u8]) -> Sha256Digest {
        self.0.fetch_add(1, Ordering::SeqCst);
        Hasher.hash_bytes(value)
    }
    fn hash_stream(&self, input: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Hasher.hash_stream(input)
    }
}

#[test]
fn combined_owner_and_nested_shape_bounds_precede_hashes_and_document_admission() {
    let original = Fixture::corrected();
    let actual = original.operation(now() - Duration::seconds(1));
    let entry = MeasureGroupEvidenceV2 {
        origin: actual.origin,
        capture: actual.group,
    };
    for mutation in 0..4 {
        let mut fixture = original.clone();
        match mutation {
            0 => fixture.material.record_history.decisions = vec![entry.clone(); 254],
            1 => fixture.material.predecessors = vec![fixture.material.predecessors[0].clone(); 33],
            2 => {
                fixture.material.result_sources =
                    vec![fixture.material.result_sources[0].clone(); 33]
            }
            _ => {
                let mut oversized = entry.clone();
                oversized.capture.measures = vec![oversized.capture.measures[0].clone(); 33];
                fixture.material.record_history.decisions.push(oversized);
            }
        }
        let hasher = Arc::new(CountHasher::default());
        let validator = Arc::new(Validator::default());
        let (service, observations) = custom_service(
            fixture.store(),
            identity(fixture.actor),
            validator.clone(),
            Arc::new(FixedClock(now())),
            hasher.clone(),
        );
        assert!(service
            .prepare("session", fixture.case_id, fixture.command)
            .is_err());
        assert_eq!(hasher.0.load(Ordering::SeqCst), 0);
        assert_eq!(validator.calls(), 0);
        assert!(observations.lock().unwrap().events.is_empty());
    }
}
