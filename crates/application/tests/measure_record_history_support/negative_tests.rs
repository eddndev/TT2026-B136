use crate::record_support::*;
use domain::{cases::CaseId, crypto::Sha256Digest};
use uuid::Uuid;

#[path = "bounds_tests.rs"]
mod bounds;
#[path = "context_tests.rs"]
mod context;
#[path = "reconstruction_tests.rs"]
mod reconstruction;

fn chain() -> (
    RecordFixture,
    MeasureAdministrativeCapture,
    MeasureRecordHistoryEvidence,
) {
    let first = RecordFixture::initial();
    let initial = first.capture();
    let second = RecordFixture::next(&initial, &first.history, 1);
    let capture = second.capture();
    let history = append_administrative(&second.history, &capture);
    (second, capture, history)
}

#[test]
fn selected_correction_requires_every_exact_administrative_and_judicial_owner() {
    let (fixture, capture, history) = chain();
    let selected = record_reference(&capture.records[0]);
    for missing in 0..3 {
        let mut incomplete = history.clone();
        if missing == 0 {
            incomplete.judicial.groups.clear();
        } else {
            incomplete.administrative.remove(missing - 1);
        }
        assert!(
            resolve_measure_records(&Hasher, fixture.case_id, &[selected], &incomplete).is_err()
        );
    }
}

fn independent_request(serial: u128) -> crate::measure_decision_fixtures::Fixture {
    let mut fixture = crate::measure_decision_fixtures::Fixture::single();
    fixture.command.operation_id =
        MeasureDecisionOperationId::from_uuid(Uuid::from_u128(1000 + serial));
    fixture.command.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(2000 + serial));
    let values = fixture.command.outcome.changes().unwrap()[0].clone();
    let MeasureEffect::Impose(mut proposal) = values else {
        unreachable!()
    };
    proposal.id = id(3000 + serial);
    fixture.command.outcome =
        MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
            MeasureEffect::Impose(proposal),
        ]))
        .unwrap();
    fixture.material.result_sources[0].id = id(3000 + serial);
    fixture
}

fn independent_group(serial: u128) -> MeasureDecisionGroupCapture {
    independent_request(serial).capture()
}

#[test]
fn mixed_selection_requires_exact_case_identity_revision_and_record_digest() {
    let (fixture, capture, evidence) = chain();
    let selected = record_reference(&capture.records[0]);
    for mutation in 0..4 {
        let mut case_id = fixture.case_id;
        let mut target = selected;
        match mutation {
            0 => case_id = CaseId::from_uuid(Uuid::from_u128(999)),
            1 => {
                target =
                    PrecautionaryMeasureRef::new(id(999), selected.revision(), selected.digest())
            }
            2 => {
                target = PrecautionaryMeasureRef::new(
                    selected.id(),
                    MeasureRevision::new(9).unwrap(),
                    selected.digest(),
                )
            }
            _ => {
                target = PrecautionaryMeasureRef::new(
                    selected.id(),
                    selected.revision(),
                    Sha256Digest::from_array([99; 32]),
                )
            }
        }
        assert!(resolve_measure_records(&Hasher, case_id, &[target], &evidence).is_err());
    }
}

#[test]
fn every_administrative_origin_field_must_identify_its_complete_receipt() {
    let (fixture, capture, evidence) = chain();
    let target = record_reference(&capture.records[0]);
    for mutation in 0..5 {
        let mut changed = evidence.clone();
        let origin = &mut changed.administrative[0].origin;
        match mutation {
            0 => origin.case_id = CaseId::from_uuid(Uuid::from_u128(999)),
            1 => {
                origin.operation_id = MeasureCorrectionOperationId::from_uuid(Uuid::from_u128(999))
            }
            2 => origin.submission_digest = Sha256Digest::from_array([99; 32]),
            3 => origin.review_digest = Sha256Digest::from_array([99; 32]),
            _ => origin.capture_digest = Sha256Digest::from_array([99; 32]),
        }
        assert!(resolve_measure_records(&Hasher, fixture.case_id, &[target], &changed).is_err());
    }
}

#[test]
fn duplicate_and_unreachable_owners_reject_even_when_each_receipt_is_valid() {
    let (fixture, capture, evidence) = chain();
    let target = record_reference(&capture.records[0]);
    for mutation in 0..4 {
        let mut changed = evidence.clone();
        match mutation {
            0 => changed
                .administrative
                .push(changed.administrative[0].clone()),
            1 => changed
                .judicial
                .groups
                .push(changed.judicial.groups[0].clone()),
            2 => {
                let extra = independent_group(9);
                changed.judicial.groups.extend(
                    crate::effect_support::append_history(
                        &crate::effect_support::empty_history(),
                        &extra,
                    )
                    .groups,
                );
            }
            _ => {
                let extra = RecordFixture::next(&capture, &fixture.history, 2).capture();
                changed = append_administrative(&changed, &extra);
            }
        }
        assert!(resolve_measure_records(&Hasher, fixture.case_id, &[target], &changed).is_err());
    }
}

#[test]
fn candidate_correction_cannot_reuse_any_supplied_owner_operation_uuid() {
    let (fixture, capture, _) = chain();
    let next = RecordFixture::next(&capture, &fixture.history, 2);
    let operations = [
        next.history.judicial.groups[0]
            .origin
            .operation_id
            .as_uuid(),
        next.history.administrative[0].origin.operation_id.as_uuid(),
        next.history.administrative[1].origin.operation_id.as_uuid(),
    ];
    for operation in operations {
        let mut changed = next.clone();
        changed.command.operation_id = MeasureCorrectionOperationId::from_uuid(operation);
        assert!(changed.prepare().is_err());
    }
}

#[test]
fn mixed_forest_rejects_cross_family_operation_uuid_ownership() {
    let fixture = RecordFixture::initial();
    let capture = fixture.capture();
    let mut judicial = crate::measure_decision_fixtures::Fixture::single();
    judicial.command.operation_id =
        MeasureDecisionOperationId::from_uuid(capture.review.command.operation_id.as_uuid());
    judicial.command.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(999));
    let MeasureEffect::Impose(mut proposal) =
        judicial.command.outcome.changes().unwrap()[0].clone()
    else {
        unreachable!()
    };
    proposal.id = id(999);
    judicial.command.outcome =
        MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
            MeasureEffect::Impose(proposal),
        ]))
        .unwrap();
    judicial.material.result_sources[0].id = id(999);
    let group = judicial.capture();
    let mut history = append_administrative(&fixture.history, &capture);
    history.judicial.groups.extend(
        crate::effect_support::append_history(&crate::effect_support::empty_history(), &group)
            .groups,
    );
    let selections = [
        record_reference(&capture.records[0]),
        reference(&group.measures[0]),
    ];
    assert!(resolve_measure_records(&Hasher, fixture.case_id, &selections, &history).is_err());
}

#[test]
fn whole_judicial_siblings_cannot_reuse_an_administrative_record_revision() {
    let initial = crate::measure_decision_fixtures::Fixture::multiple().capture();
    let first = RecordFixture::from_first(CorrectionFixture::from_group(
        &initial,
        &crate::effect_support::empty_history(),
        id(70),
    ));
    let administrative = first.capture();
    let mut later = crate::effect_support::LaterFixture::confirm(&initial);
    later.effects(
        initial
            .measures
            .iter()
            .map(|row| MeasureEffect::Confirm {
                previous: reference(row),
            })
            .collect(),
    );
    later.request.material.predecessors = initial
        .measures
        .iter()
        .map(|row| crate::effect_support::owned_member(&initial, row))
        .collect();
    later.request.material.result_sources = initial
        .measures
        .iter()
        .map(|row| MeasureResultSources {
            id: row.result.id,
            sources: row.result.sources.clone(),
        })
        .collect();
    let judicial = later.capture();
    let mut history = append_administrative(&first.history, &administrative);
    history.judicial = crate::effect_support::append_history(&first.history.judicial, &judicial);
    let other = judicial
        .measures
        .iter()
        .find(|row| row.result.id == id(80))
        .unwrap();
    let selected = [
        record_reference(&administrative.records[0]),
        reference(other),
    ];
    assert!(resolve_measure_records(&Hasher, first.case_id, &selected, &history).is_err());
}
