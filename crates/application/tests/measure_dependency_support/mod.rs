pub use crate::decision_review_support::*;
pub use crate::measure_decision_fixtures::Fixture;
pub use application::measure_corrections::{
    inspect_measure_administrative_dependencies, CheckedMeasureAdministrativeDependencies,
    MeasureAdministrativeDependant, MeasureAdministrativeDependencyInventory,
    MeasureAdministrativeHearingHistory, MeasureDependencyHearingRef,
    MeasureDependencyJudicialOwner,
};
use domain::cases::CaseId;
use time::Duration;
use uuid::Uuid;

pub fn judicial_inventory(
    group: &MeasureDecisionGroupCapture,
) -> MeasureAdministrativeDependencyInventory {
    let mut records = empty_decision_history();
    records.records.judicial =
        crate::effect_support::append_history(&crate::effect_support::empty_history(), group);
    MeasureAdministrativeDependencyInventory {
        records,
        hearings: vec![],
    }
}

pub fn inspect(
    target: PrecautionaryMeasureRef,
    inventory: &MeasureAdministrativeDependencyInventory,
) -> CheckedMeasureAdministrativeDependencies {
    inspect_measure_administrative_dependencies(
        &Hasher,
        CaseId::from_uuid(Uuid::from_u128(1)),
        target,
        inventory,
    )
    .unwrap()
}

pub fn owner_v1(group: &MeasureDecisionGroupCapture) -> MeasureDependencyJudicialOwner {
    MeasureDependencyJudicialOwner::V1(MeasureGroupRef {
        operation_id: group.decision.operation_id,
        decision_id: group.decision.decision_id,
        group_digest: group.capture_digest,
    })
}

pub fn owner_v2(group: &MeasureDecisionGroupCaptureV2) -> MeasureDependencyJudicialOwner {
    MeasureDependencyJudicialOwner::V2(MeasureGroupRef {
        operation_id: group.decision.operation_id,
        decision_id: group.decision.decision_id,
        group_digest: group.capture_digest,
    })
}

pub fn administrative_owner(capture: &MeasureAdministrativeCapture) -> MeasureAdministrativeRef {
    MeasureAdministrativeRef {
        operation_id: capture.review.command.operation_id,
        capture_digest: capture.capture_digest,
    }
}

pub fn hearing_ref(capture: &PrecautionaryHearingCapture) -> MeasureDependencyHearingRef {
    MeasureDependencyHearingRef {
        hearing_id: capture.review.command.hearing_id,
        revision: capture.review.result_revision,
        operation_id: capture.review.command.operation_id,
        capture_digest: capture.capture_digest,
    }
}

pub fn hearing_prefix(
    captures: Vec<PrecautionaryHearingCapture>,
    initial_history: &MeasureDecisionRecordHistoryEvidence,
) -> MeasureAdministrativeHearingHistory {
    MeasureAdministrativeHearingHistory {
        origin: precautionary_hearing_origin_with_decision_history(
            &Hasher,
            &captures[0],
            initial_history,
        )
        .unwrap(),
        captures,
    }
}

pub fn anchor_v1(
    hearing: &PrecautionaryHearingCapture,
    history: &MeasureHistoryEvidence,
    serial: u128,
) -> MeasureDecisionGroupCapture {
    let mut fixture = Fixture::no_change();
    fixture.command.operation_id =
        MeasureDecisionOperationId::from_uuid(Uuid::from_u128(5000 + serial));
    fixture.command.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(6000 + serial));
    fixture.material.context = hearing.review.observed_context.clone();
    fixture.command.context = expectation(&fixture.material.context);
    fixture.command.anchor = Some(MeasureDecisionAnchorRef::Precautionary {
        hearing_id: hearing.review.command.hearing_id,
        revision: hearing.review.result_revision,
        capture_digest: hearing.capture_digest,
    });
    fixture.material.anchor = Some(MeasureDecisionAnchorMaterial::Precautionary(Box::new(
        hearing.clone(),
    )));
    prepare_measure_decision_with_history(
        &Hasher,
        &fixture.actor,
        fixture.case_id,
        fixture.command,
        fixture.material,
        history,
    )
    .unwrap()
    .into_group_capture(&Hasher, hearing.recorded_at + Duration::seconds(1))
    .unwrap()
}

pub fn anchor_v2(
    hearing: &PrecautionaryHearingCapture,
    history: &MeasureDecisionRecordHistoryEvidence,
    serial: u128,
) -> MeasureDecisionGroupCaptureV2 {
    let mut fixture = FixtureV2::initial(Fixture::no_change());
    fixture.identities(serial);
    fixture.material.context = hearing.review.observed_context.clone();
    fixture.command.context = expectation(&fixture.material.context);
    fixture.command.anchor = Some(MeasureDecisionAnchorRef::Precautionary {
        hearing_id: hearing.review.command.hearing_id,
        revision: hearing.review.result_revision,
        capture_digest: hearing.capture_digest,
    });
    fixture.material.anchor = Some(MeasureDecisionAnchorMaterial::Precautionary(Box::new(
        hearing.clone(),
    )));
    fixture.history = history.clone();
    fixture.recorded_at = hearing.recorded_at + Duration::seconds(1);
    fixture.capture()
}

pub fn independent_request(serial: u128, measure: u128) -> Fixture {
    let mut fixture = Fixture::single();
    let MeasureEffect::Impose(mut proposal) = fixture.command.outcome.changes().unwrap()[0].clone()
    else {
        panic!("imposition expected")
    };
    proposal.id = id(measure);
    fixture.command.operation_id =
        MeasureDecisionOperationId::from_uuid(Uuid::from_u128(7000 + serial));
    fixture.command.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(8000 + serial));
    fixture.command.outcome =
        MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
            MeasureEffect::Impose(proposal),
        ]))
        .unwrap();
    fixture.material.result_sources[0].id = id(measure);
    fixture
}
