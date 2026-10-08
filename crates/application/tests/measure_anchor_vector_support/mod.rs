mod literals;

pub use literals::*;

use application::cases::CaseActorSnapshot;
use application::hearings::*;
use application::participants::ParticipantOverview;
use application::precautionary_hearings::*;
use application::precautionary_measures::*;
use domain::case_stages::CaseStage;
use domain::crypto::Sha256Digest;
use domain::participants::{DirectoryStatus, ParticipantId, ParticipantRevision};
use domain::precautionary_hearings::*;
use domain::typed_participants::ParticipantKind;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::decision_vector_support::{capture, Hasher};

fn note(value: &str) -> HearingNote {
    HearingNote::new(value).unwrap()
}

fn time() -> HearingTime {
    HearingTime::new(OffsetDateTime::from_unix_timestamp(60).unwrap()).unwrap()
}

fn initial(base: &MeasureDecisionGroupCapture) -> HearingDetail {
    let context = base.decision.context.material();
    let participant = HearingParticipantRef::new(
        ParticipantId::from_uuid(Uuid::from_u128(22)),
        ParticipantRevision::new(2).unwrap(),
    );
    let values = HearingValues::new(HearingValuesInput {
        kind: HearingKind::Initial,
        scheduled_at: time(),
        modality: HearingModality::InPerson,
        venue: HearingVenue::new("C").unwrap(),
        note: Some(note("N")),
        participants: vec![participant],
        conviction_basis: None,
    })
    .unwrap();
    let expected_context = HearingContextExpectation {
        case_revision: context.administration.revision,
        stage_revision: context.stage.stage_revision(),
    };
    let command = HearingCommand {
        operation_id: HearingOperationId::from_uuid(Uuid::from_u128(30)),
        hearing_id: HearingId::from_uuid(Uuid::from_u128(31)),
        change: HearingChange::Schedule {
            context: expected_context,
            values: values.clone(),
        },
    };
    let values_digest = hearing_values_digest(&Hasher, &values);
    let submission_digest = hearing_submission_digest(
        &Hasher,
        base.review.actor.id,
        base.review.case_id,
        &command,
        values_digest,
    );
    HearingDetail {
        snapshot: HearingSnapshot {
            case_id: base.review.case_id,
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
                expected_context: Some(expected_context),
                submission_digest,
            },
            scheduling_context: HearingSchedulingContext {
                administration_revision: context.administration.revision,
                administration_digest: context.administration.values_digest,
                stage_revision: context.stage.stage_revision(),
                stage: CaseStage::Investigation,
                stage_digest: None,
            },
            recorded_administration_revision: context.administration.revision,
            recorded_administration_digest: context.administration.values_digest,
            recorded_at: OffsetDateTime::from_unix_timestamp_nanos(8).unwrap(),
            recorded_by: CaseActorSnapshot {
                id: base.review.actor.id,
                email: base.review.actor.email.clone(),
            },
        },
        participants: vec![HearingParticipantSnapshot {
            overview: ParticipantOverview {
                case_id: base.review.case_id,
                id: participant.id(),
                revision: participant.revision(),
                display_name: "N\u{e9}".into(),
                procedural_role: "R".into(),
                organization: Some("O".into()),
                directory_status: DirectoryStatus::Archived,
                kind: Some(ParticipantKind::TrialCourt),
                subject: Some(base.measures[0].result.values.subject()),
            },
            values_digest: Sha256Digest::from_array([0x44; 32]),
        }],
        support: None,
    }
}

fn precautionary(base: &MeasureDecisionGroupCapture) -> PrecautionaryHearingCapture {
    let command = PrecautionaryHearingCommand {
        operation_id: PrecautionaryHearingOperationId::from_uuid(Uuid::from_u128(20)),
        hearing_id: PrecautionaryHearingId::from_uuid(Uuid::from_u128(21)),
        change: PrecautionaryHearingChange::Schedule {
            context: base.review.command.context,
            values: PrecautionaryHearingValues::new(PrecautionaryHearingValuesInput {
                purpose: PrecautionaryHearingPurpose::Imposition,
                scheduled_at: time(),
                modality: HearingModality::InPerson,
                venue: HearingVenue::new("C").unwrap(),
                note: None,
                participants: vec![],
                scheduling_basis: PrecautionaryHearingSchedulingBasis::new(
                    note("B"),
                    base.review.command.values.support(),
                    note("L"),
                ),
                review_targets: vec![],
            })
            .unwrap(),
        },
    };
    prepare_precautionary_hearing_capture(
        &Hasher,
        &base.review.actor,
        base.review.case_id,
        command,
        base.decision.context.clone(),
        PrecautionaryHearingSources {
            participants: vec![],
            support: base.decision.support.clone(),
        },
        None,
    )
    .unwrap()
    .into_capture(
        &Hasher,
        OffsetDateTime::from_unix_timestamp_nanos(8).unwrap(),
    )
    .unwrap()
}

pub fn anchored(initial_family: bool) -> (MeasureDecisionCommand, MeasureDecisionCapture) {
    let base = capture(false);
    let mut command = base.review.command.clone();
    let mut decision = base.decision.clone();
    if initial_family {
        let detail = initial(&base);
        hearing_receipt_matches(&Hasher, &detail).unwrap();
        command.anchor = Some(MeasureDecisionAnchorRef::Initial {
            hearing_id: detail.snapshot.id,
            revision: detail.snapshot.revision,
            values_digest: detail.snapshot.values_digest,
            submission_digest: detail.snapshot.receipt.submission_digest,
        });
        decision.anchor = Some(MeasureDecisionAnchorMaterial::Initial(Box::new(detail)));
    } else {
        let detail = precautionary(&base);
        precautionary_hearing_receipt_matches(&Hasher, &detail).unwrap();
        command.anchor = Some(MeasureDecisionAnchorRef::Precautionary {
            hearing_id: detail.review.command.hearing_id,
            revision: detail.review.result_revision,
            capture_digest: detail.capture_digest,
        });
        decision.anchor = Some(MeasureDecisionAnchorMaterial::Precautionary(Box::new(
            detail,
        )));
    }
    (command, decision)
}
