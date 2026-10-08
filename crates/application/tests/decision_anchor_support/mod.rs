use crate::decision_support::*;
use application::hearings::*;
use application::ApplicationError;
use domain::clock::OffsetDateTime;
use uuid::Uuid;

pub fn empty() -> MeasureHistoryEvidence {
    MeasureHistoryEvidence { groups: vec![] }
}

pub fn prepare(
    fixture: Fixture,
    evidence: &MeasureHistoryEvidence,
) -> Result<CheckedMeasureDecisionReview, ApplicationError> {
    prepare_measure_decision_with_history(
        &Hasher,
        &fixture.actor,
        fixture.case_id,
        fixture.command,
        fixture.material,
        evidence,
    )
}

pub fn capture(
    fixture: Fixture,
    evidence: &MeasureHistoryEvidence,
    at: OffsetDateTime,
) -> MeasureDecisionGroupCapture {
    prepare(fixture, evidence)
        .unwrap()
        .into_group_capture(&Hasher, at)
        .unwrap()
}

pub fn attach_initial(fixture: &mut Fixture, detail: HearingDetail) {
    fixture.command.anchor = Some(MeasureDecisionAnchorRef::Initial {
        hearing_id: detail.snapshot.id,
        revision: detail.snapshot.revision,
        values_digest: detail.snapshot.values_digest,
        submission_digest: detail.snapshot.receipt.submission_digest,
    });
    fixture.material.anchor = Some(MeasureDecisionAnchorMaterial::Initial(Box::new(detail)));
}

pub fn ordinary_initial() -> HearingDetail {
    let fixture = Fixture::single();
    let material = fixture.material.context.material();
    let values = HearingValues::new(HearingValuesInput {
        kind: HearingKind::Initial,
        scheduled_at: HearingTime::new(at().replace_nanosecond(0).unwrap()).unwrap(),
        modality: HearingModality::InPerson,
        venue: HearingVenue::new("Initial court").unwrap(),
        note: None,
        participants: vec![],
        conviction_basis: None,
    })
    .unwrap();
    let context = HearingContextExpectation {
        case_revision: material.administration.revision,
        stage_revision: material.stage.stage_revision(),
    };
    let command = HearingCommand {
        operation_id: HearingOperationId::from_uuid(Uuid::from_u128(120)),
        hearing_id: HearingId::from_uuid(Uuid::from_u128(121)),
        change: HearingChange::Schedule {
            context,
            values: values.clone(),
        },
    };
    let values_digest = hearing_values_digest(&Hasher, &values);
    HearingDetail {
        snapshot: HearingSnapshot {
            case_id: fixture.case_id,
            id: command.hearing_id,
            revision: HearingRevision::initial(),
            values,
            values_digest,
            status: HearingStatus::Scheduled,
            reason: None,
            receipt: HearingReceipt {
                operation_id: command.operation_id,
                action: HearingAction::Schedule,
                expected_revision: 0,
                expected_context: Some(context),
                submission_digest: hearing_submission_digest(
                    &Hasher,
                    fixture.actor.id,
                    fixture.case_id,
                    &command,
                    values_digest,
                ),
            },
            scheduling_context: HearingSchedulingContext {
                administration_revision: material.administration.revision,
                administration_digest: material.administration.values_digest,
                stage_revision: material.stage.stage_revision(),
                stage: material.stage.stage(),
                stage_digest: None,
            },
            recorded_administration_revision: material.administration.revision,
            recorded_administration_digest: material.administration.values_digest,
            recorded_at: at(),
            recorded_by: material.administration.changed_by.clone(),
        },
        participants: vec![],
        support: None,
    }
}

pub fn refresh_ordinary(detail: &mut HearingDetail) {
    let snapshot = &mut detail.snapshot;
    snapshot.values_digest = hearing_values_digest(&Hasher, &snapshot.values);
    let change = match snapshot.receipt.action {
        HearingAction::Schedule => HearingChange::Schedule {
            context: snapshot.receipt.expected_context.unwrap(),
            values: snapshot.values.clone(),
        },
        HearingAction::Replace => HearingChange::Replace {
            expected_revision: HearingRevision::new(snapshot.receipt.expected_revision).unwrap(),
            context: snapshot.receipt.expected_context.unwrap(),
            values: snapshot.values.clone(),
            reason: snapshot.reason.clone().unwrap(),
        },
        HearingAction::Cancel => HearingChange::Cancel {
            expected_revision: HearingRevision::new(snapshot.receipt.expected_revision).unwrap(),
            reason: snapshot.reason.clone().unwrap(),
        },
    };
    let command = HearingCommand {
        operation_id: snapshot.receipt.operation_id,
        hearing_id: snapshot.id,
        change,
    };
    snapshot.receipt.submission_digest = hearing_submission_digest(
        &Hasher,
        snapshot.recorded_by.id,
        snapshot.case_id,
        &command,
        snapshot.values_digest,
    );
}

pub fn cancelled_initial() -> HearingDetail {
    let mut detail = ordinary_initial();
    let snapshot = &mut detail.snapshot;
    snapshot.revision = HearingRevision::new(2).unwrap();
    snapshot.status = HearingStatus::Cancelled;
    snapshot.reason = Some(HearingNote::new("Historical cancellation").unwrap());
    snapshot.receipt.operation_id = HearingOperationId::from_uuid(Uuid::from_u128(122));
    snapshot.receipt.action = HearingAction::Cancel;
    snapshot.receipt.expected_revision = 1;
    snapshot.receipt.expected_context = None;
    refresh_ordinary(&mut detail);
    detail
}

pub fn fresh_no_change(serial: u128) -> Fixture {
    let mut fixture = Fixture::no_change();
    fixture.command.operation_id =
        MeasureDecisionOperationId::from_uuid(Uuid::from_u128(1000 + serial));
    fixture.command.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(2000 + serial));
    fixture
}

pub fn attach_precautionary(
    fixture: &mut Fixture,
    hearing: application::precautionary_hearings::PrecautionaryHearingCapture,
) {
    fixture.material.context = hearing.review.observed_context.clone();
    fixture.command.context = expectation(&fixture.material.context);
    fixture.command.anchor = Some(MeasureDecisionAnchorRef::Precautionary {
        hearing_id: hearing.review.command.hearing_id,
        revision: hearing.review.result_revision,
        capture_digest: hearing.capture_digest,
    });
    fixture.material.anchor = Some(MeasureDecisionAnchorMaterial::Precautionary(Box::new(
        hearing,
    )));
}

pub fn review_hearing(
    serial: u128,
    targets: &[PrecautionaryMeasureRef],
    evidence: &MeasureHistoryEvidence,
    recorded_at: OffsetDateTime,
) -> application::precautionary_hearings::PrecautionaryHearingCapture {
    use application::precautionary_hearings::*;
    use domain::precautionary_hearings::*;
    let mut hearing = crate::precautionary_receipt_support::Fixture::schedule();
    let mut input = crate::precautionary_receipt_support::values_input();
    input.purpose = PrecautionaryHearingPurpose::Review;
    input.review_targets = targets.to_vec();
    input.participants.clear();
    hearing.sources.participants.clear();
    hearing.command.operation_id =
        PrecautionaryHearingOperationId::from_uuid(Uuid::from_u128(3000 + serial));
    hearing.command.hearing_id = PrecautionaryHearingId::from_uuid(Uuid::from_u128(4000 + serial));
    hearing.command.change = PrecautionaryHearingChange::Schedule {
        context: crate::precautionary_receipt_support::expectation(&hearing.context),
        values: PrecautionaryHearingValues::new(input).unwrap(),
    };
    prepare_precautionary_hearing_with_history(
        &Hasher,
        &hearing.actor,
        hearing.case_id,
        hearing.command,
        PrecautionaryHearingPreparationMaterial {
            observed_context: hearing.context,
            sources: hearing.sources,
            predecessor: None,
            measure_history: evidence,
        },
    )
    .unwrap()
    .into_capture(&Hasher, recorded_at)
    .unwrap()
}
