use super::*;
use domain::DomainError;
use std::io::Read;
use time::OffsetDateTime;

// Deliberate receipt collisions isolate graph validation from hash collision resistance.
struct ReceiptCollisionHasher;
impl DocumentHasher for ReceiptCollisionHasher {
    fn hash_bytes(&self, bytes: &[u8]) -> Sha256Digest {
        if bytes.starts_with(b"PHCR1") {
            Sha256Digest::from_array([0xa5; 32])
        } else {
            Hasher.hash_bytes(bytes)
        }
    }
    fn hash_stream(&self, source: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes).unwrap();
        Ok(self.hash_bytes(&bytes))
    }
}

fn hearing(
    fixture: HearingFixture,
    predecessor: Option<&PrecautionaryHearingCapture>,
    records: &MeasureDecisionRecordHistoryEvidence,
    recorded_at: OffsetDateTime,
) -> PrecautionaryHearingCapture {
    prepare_precautionary_hearing_with_decision_history(
        &ReceiptCollisionHasher,
        &fixture.actor,
        fixture.case_id,
        fixture.command,
        PrecautionaryHearingDecisionPreparationMaterial {
            observed_context: fixture.context,
            sources: fixture.sources,
            predecessor,
            decision_history: records,
        },
    )
    .unwrap()
    .into_capture(&ReceiptCollisionHasher, recorded_at)
    .unwrap()
}

#[test]
fn a_group_cannot_depend_on_itself_through_the_earlier_review_in_its_anchor_prefix() {
    let base = Fixture::single().capture();
    let base_target = reference(&base.measures[0]);
    let mut inventory = judicial_inventory(&base);
    let mut scheduled = HearingFixture::schedule();
    crate::record_review_support::select_targets(&mut scheduled, vec![base_target]);
    let first = hearing(scheduled, None, &inventory.records, base.recorded_at);
    let mut replaced = HearingFixture::replace(&first);
    set_context(&mut replaced, first.review.observed_context.clone());
    let second = hearing(replaced, Some(&first), &inventory.records, base.recorded_at);
    assert!(second.review.resolved_values.review_targets().is_empty());

    let mut request = FixtureV2::initial(independent_request(1, 80));
    request.command.anchor = Some(MeasureDecisionAnchorRef::Precautionary {
        hearing_id: second.review.command.hearing_id,
        revision: second.review.result_revision,
        capture_digest: second.capture_digest,
    });
    request.material.anchor = Some(MeasureDecisionAnchorMaterial::Precautionary(Box::new(
        second.clone(),
    )));
    let group = prepare_measure_decision_with_record_history(
        &ReceiptCollisionHasher,
        &request.actor,
        request.case_id,
        request.command,
        request.material,
        &empty_decision_history(),
    )
    .unwrap()
    .into_group_capture(&ReceiptCollisionHasher, base.recorded_at)
    .unwrap();
    let origin =
        measure_group_origin_v2(&ReceiptCollisionHasher, &group, &empty_decision_history())
            .unwrap();
    inventory.records.decisions.push(MeasureGroupEvidenceV2 {
        origin,
        capture: group.clone(),
    });
    let hearing_origin = precautionary_hearing_origin_with_decision_history(
        &ReceiptCollisionHasher,
        &first,
        &judicial_inventory(&base).records,
    )
    .unwrap();
    inventory
        .hearings
        .push(MeasureAdministrativeHearingHistory {
            origin: hearing_origin,
            captures: vec![first, second],
        });
    assert!(inspect_measure_administrative_dependencies(
        &ReceiptCollisionHasher,
        base.review.case_id,
        base_target,
        &inventory,
    )
    .is_ok());

    let self_target = reference_v2(&group.measures[0]);
    let first = &mut inventory.hearings[0].captures[0];
    let mut input = crate::record_review_support::values_input(&first.review.resolved_values);
    input.review_targets = vec![self_target];
    let values = PrecautionaryHearingValues::new(input).unwrap();
    let PrecautionaryHearingChange::Schedule {
        values: scheduled, ..
    } = &mut first.review.command.change
    else {
        panic!("initial schedule required")
    };
    *scheduled = values.clone();
    first.review.resolved_values = values;
    first.review.submission_digest = ReceiptCollisionHasher.hash_bytes(
        &precautionary_hearing_submission_bytes(
            &first.review.actor,
            first.review.case_id,
            &first.review.command,
            &first.review.resolved_values,
        )
        .unwrap(),
    );
    first.review.review_digest = ReceiptCollisionHasher
        .hash_bytes(&precautionary_hearing_review_bytes(&first.review).unwrap());
    assert_eq!(
        first.capture_digest,
        ReceiptCollisionHasher.hash_bytes(&precautionary_hearing_capture_bytes(first).unwrap())
    );
    let mut selected_closure = empty_decision_history();
    selected_closure.decisions = inventory.records.decisions.clone();
    let exact_origin = precautionary_hearing_origin_with_decision_history(
        &ReceiptCollisionHasher,
        first,
        &selected_closure,
    )
    .unwrap();
    inventory.hearings[0].origin = exact_origin;
    assert!(precautionary_hearing_history_with_decision_history_matches(
        &ReceiptCollisionHasher,
        &inventory.hearings[0].captures,
        &inventory.hearings[0].origin,
        &selected_closure,
    )
    .is_ok());
    assert!(inspect_measure_administrative_dependencies(
        &ReceiptCollisionHasher,
        base.review.case_id,
        base_target,
        &inventory,
    )
    .is_err());
}
