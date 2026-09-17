use super::*;
use application::{
    case_stages::*,
    participants::{DirectoryStatus, ParticipantOverview},
};
use domain::{
    crypto::Sha256Digest,
    participants::{ParticipantId, ParticipantRevision},
};

pub fn trial(preparation: &mut HearingPreparation, actor: UserId, record: &DocumentRecord) {
    let change = CaseStageChange::Adopt(StageAdoption::new(
        CaseStage::Trial,
        DeclaredStageTime::instant(instant()).unwrap(),
        StageNote::new("Declared prior trial").unwrap(),
        StageSupportRef::new(support(record).reference(), record.digest),
    ));
    preparation.context.stage = CurrentCaseStage::Registered(Box::new(CaseStageEntry::Changed(
        Box::new(CaseStageSnapshot {
            case_id: preparation.context.case_id,
            stage_revision: CaseStageRevision::new(2).unwrap(),
            from_stage: None,
            values_digest: case_stage_digest(hasher().as_ref(), &change),
            values: change,
            administration_revision: CaseRevision::FIRST,
            administration_digest: preparation
                .context
                .administration
                .snapshot()
                .unwrap()
                .values_digest,
            supports: vec![],
            recorded_at: instant(),
            recorded_by: CaseActorSnapshot {
                id: actor,
                email: "actor@example.com".into(),
            },
        }),
    )));
}

pub fn sentencing(record: &DocumentRecord) -> HearingCommand {
    let mut input = values_input(&values());
    input.kind = HearingKind::Sentencing;
    input.conviction_basis = Some(HearingConvictionBasis::new(
        HearingNote::new("Operator declared conviction").unwrap(),
        support(record),
    ));
    HearingCommand {
        operation_id: HearingOperationId::new(),
        hearing_id: HearingId::new(),
        change: HearingChange::Schedule {
            context: HearingContextExpectation {
                case_revision: CaseRevision::FIRST,
                stage_revision: CaseStageRevision::new(2).unwrap(),
            },
            values: HearingValues::new(input).unwrap(),
        },
    }
}

pub fn command_values(command: &HearingCommand) -> HearingValues {
    match &command.change {
        HearingChange::Schedule { values, .. } | HearingChange::Replace { values, .. } => {
            values.clone()
        }
        HearingChange::Cancel { .. } => panic!("cancellation derives its values"),
    }
}

pub fn participant(
    case_id: CaseId,
    id: u128,
    status: DirectoryStatus,
) -> HearingParticipantSnapshot {
    HearingParticipantSnapshot {
        overview: ParticipantOverview {
            case_id,
            id: ParticipantId::from_uuid(uuid::Uuid::from_u128(id)),
            revision: ParticipantRevision::initial(),
            display_name: "Declared person".into(),
            procedural_role: "Defense counsel".into(),
            organization: None,
            directory_status: status,
            kind: None,
            subject: None,
        },
        values_digest: Sha256Digest::from_array([id as u8; 32]),
    }
}

pub fn selected(command: &mut HearingCommand, participants: &[HearingParticipantSnapshot]) {
    let mut input = values_input(&command_values(command));
    input.participants = participants
        .iter()
        .map(|p| HearingParticipantRef::new(p.overview.id, p.overview.revision))
        .collect();
    match &mut command.change {
        HearingChange::Schedule { values, .. } | HearingChange::Replace { values, .. } => {
            *values = HearingValues::new(input).unwrap()
        }
        _ => panic!("cannot select for cancellation"),
    }
}
