use super::*;
use domain::crypto::{DocumentId, DocumentVersionRef, Sha256Digest};
use domain::hearings::{HearingNote, HearingSupportRef};
use uuid::Uuid;

#[test]
fn a_supervisor_removed_then_reintroduced_cannot_change_its_exact_historical_provenance() {
    let first = Fixture::single().capture();
    let mut removed = LaterFixture::confirm(&first);
    let mut input = values_input(&first.measures[0].result.values);
    input.supervision = MeasureSupervision::Unknown {
        reason: HearingNote::new("Supervisor omitted by the later declaration").unwrap(),
    };
    removed.effects(vec![MeasureEffect::Modify {
        previous: reference(&first.measures[0]),
        values: MeasureValues::new(input),
    }]);
    removed.request.material.result_sources[0]
        .sources
        .supervisor = None;
    let first_evidence = removed.evidence.clone();
    let second = removed.capture();
    for changed in [false, true] {
        let mut returning = LaterFixture::next(&second, &first_evidence, 2);
        returning.request.material.result_sources[0].sources =
            first.measures[0].result.sources.clone();
        if changed {
            crate::participant_support::typed_mut(
                returning.request.material.result_sources[0]
                    .sources
                    .supervisor
                    .as_mut()
                    .unwrap(),
            )
            .changed_by
            .email = "rewritten@example.test".into();
        }
        let values = first.measures[0].result.values.clone();
        resolve_measure_sources(
            &Hasher,
            returning.request.case_id,
            &values,
            &returning.request.material.result_sources[0].sources,
        )
        .unwrap();
        returning.effects(vec![MeasureEffect::Modify {
            previous: reference(&second.measures[0]),
            values,
        }]);
        if changed {
            assert!(returning.prepare().is_err());
        } else {
            let evidence = returning.evidence.clone();
            let third = returning.capture();
            measure_decision_group_with_history_matches(&Hasher, &third, &evidence).unwrap();
        }
    }
}

#[test]
fn a_decision_support_version_cannot_return_with_changed_metadata_after_an_intermediate_support() {
    let first = Fixture::single().capture();
    let mut intermediate = LaterFixture::confirm(&first);
    let support = &mut intermediate.request.material.support;
    support.reference = DocumentVersionRef {
        id: DocumentId::from_uuid(Uuid::from_u128(92)),
        version: support.reference.version,
    };
    support.digest = Sha256Digest::from_array([12; 32]);
    support.name = "intermediate.pdf".into();
    let mut values = decision_input(&intermediate.request.command.values);
    values.support = HearingSupportRef::new(support.reference, support.digest);
    intermediate.request.command.values = MeasureDecisionValues::new(values);
    let first_evidence = intermediate.evidence.clone();
    let second = intermediate.capture();
    for changed in [false, true] {
        let mut returning = LaterFixture::next(&second, &first_evidence, 2);
        returning.request.material.support = first.decision.support.clone();
        if changed {
            returning.request.material.support.name = "rewritten.pdf".into();
        }
        let mut values = decision_input(&returning.request.command.values);
        values.support = HearingSupportRef::new(
            returning.request.material.support.reference,
            returning.request.material.support.digest,
        );
        returning.request.command.values = MeasureDecisionValues::new(values);
        if changed {
            assert!(returning.prepare().is_err());
        } else {
            let evidence = returning.evidence.clone();
            let third = returning.capture();
            measure_decision_group_with_history_matches(&Hasher, &third, &evidence).unwrap();
        }
    }
}
