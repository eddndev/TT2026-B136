use super::validation_receipt::{inconsistent, validate_participant_projection};
use super::*;
use crate::{
    case_stages::{case_stage_digest, CaseStageEntry},
    cases::case_administration_digest,
    participants::DirectoryStatus,
    ApplicationError,
};
use domain::{
    case_administration::CaseAdministrativeStatus, cases::CaseId, crypto::DocumentHasher,
};

pub(super) fn validate_context(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    context: &HearingCaseContext,
) -> Result<(), ApplicationError> {
    if context.case_id != case_id {
        return Err(inconsistent("prepared context belongs to another case"));
    }
    if let Some(admin) = context.administration.snapshot() {
        if admin.case_id != case_id
            || case_administration_digest(hasher, &admin.values) != admin.values_digest
        {
            return Err(inconsistent(
                "prepared administration scope or digest differs",
            ));
        }
    }
    if let Some(stage) = context.stage.entry() {
        if stage.case_id() != case_id {
            return Err(inconsistent("prepared stage belongs to another case"));
        }
        match stage {
            CaseStageEntry::Initial(value) => {
                if value.stage_revision.get() != 1 || value.administration_revision.get() != 1 {
                    return Err(inconsistent("initial stage revisions are not one"));
                }
                if let Some(admin) = context
                    .administration
                    .snapshot()
                    .filter(|admin| admin.revision == value.administration_revision)
                {
                    if admin.values_digest != value.administration_digest {
                        return Err(inconsistent("initial stage administration digest differs"));
                    }
                }
            }
            CaseStageEntry::Changed(value) => {
                if case_stage_digest(hasher, &value.values) != value.values_digest {
                    return Err(inconsistent("prepared stage digest differs"));
                }
            }
        }
    }
    Ok(())
}

pub(super) fn validate_preparation(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    command: &HearingCommand,
    preparation: &mut HearingPreparation,
) -> Result<(HearingValues, HearingSchedulingContext), ApplicationError> {
    validate_context(hasher, case_id, &preparation.context)?;
    if preparation.context.administration.values().status() != CaseAdministrativeStatus::Active {
        return Err(ApplicationError::CaseClosed);
    }
    let base = preparation.base.as_ref();
    match (&command.change, base) {
        (HearingChange::Schedule { .. }, Some(_)) => {
            return Err(HearingError::RevisionConflict.into());
        }
        (HearingChange::Replace { .. } | HearingChange::Cancel { .. }, None) => {
            return Err(HearingError::NotFound.into());
        }
        _ => {}
    }
    if let Some(base) = base {
        if base.snapshot.case_id != case_id || base.snapshot.id != command.hearing_id {
            return Err(inconsistent("prepared base belongs to another hearing"));
        }
        if base.snapshot.revision.get() != command.expected_revision() {
            return Err(HearingError::RevisionConflict.into());
        }
        if base.snapshot.status == HearingStatus::Cancelled {
            return Err(HearingError::AlreadyCancelled.into());
        }
        hearing_receipt_matches(hasher, base)?;
        let current_admin = preparation
            .context
            .administration
            .snapshot()
            .ok_or_else(|| inconsistent("existing hearing has no recorded administration"))?;
        if current_admin.revision < base.snapshot.recorded_administration_revision
            || (current_admin.revision == base.snapshot.recorded_administration_revision
                && current_admin.values_digest != base.snapshot.recorded_administration_digest)
        {
            return Err(inconsistent(
                "prepared administration predates or contradicts the hearing",
            ));
        }
        if base.snapshot.receipt.operation_id == command.operation_id {
            return Err(HearingError::OperationConflict.into());
        }
    }
    if matches!(command.change, HearingChange::Cancel { .. }) {
        let base = base.ok_or_else(|| inconsistent("cancel has no exact base"))?;
        if preparation.participants != base.participants || !preparation.records.is_empty() {
            return Err(inconsistent(
                "cancel preparation does not preserve historical sources",
            ));
        }
        return Ok((
            base.snapshot.values.clone(),
            base.snapshot.scheduling_context,
        ));
    }
    let (context, values) = match &command.change {
        HearingChange::Schedule { context, values }
        | HearingChange::Replace {
            context, values, ..
        } => (*context, values),
        HearingChange::Cancel { .. } => unreachable!("cancel returned above"),
    };
    let admin = preparation
        .context
        .administration
        .snapshot()
        .ok_or(HearingError::ContextRequired)?;
    let stage = preparation
        .context
        .stage
        .entry()
        .ok_or(HearingError::ContextRequired)?;
    if admin.values.profile().is_none() {
        return Err(HearingError::ContextRequired.into());
    }
    if context.case_revision != admin.revision || context.stage_revision != stage.stage_revision() {
        return Err(HearingError::ContextConflict.into());
    }
    if base.is_some_and(|base| base.snapshot.values.kind() != values.kind()) {
        return Err(HearingError::ImmutableKind.into());
    }
    if stage.stage() != values.kind().required_stage() {
        return Err(HearingError::StageIncompatible.into());
    }
    if preparation.participants.len() != values.participants().len() {
        return Err(inconsistent(
            "prepared participant projection count differs",
        ));
    }
    preparation
        .participants
        .sort_by_key(|participant| participant.overview.id.as_uuid());
    validate_participant_projection(case_id, values, &preparation.participants)?;
    for participant in &preparation.participants {
        let retained = base.and_then(|base| {
            base.participants.iter().find(|old| {
                old.overview.id == participant.overview.id
                    && old.overview.revision == participant.overview.revision
            })
        });
        if let Some(retained) = retained {
            if retained != participant {
                return Err(inconsistent("retained participant projection changed"));
            }
        } else if participant.overview.directory_status != DirectoryStatus::Active {
            return Err(HearingError::ParticipantChanged.into());
        }
    }
    let support = values.conviction_basis().map(|basis| basis.support());
    match (support, preparation.records.as_slice()) {
        (None, []) => {}
        (Some(support), [record])
            if record.id == support.reference().id
                && record.version == support.reference().version =>
        {
            if record.digest != support.digest() {
                return Err(HearingError::SupportDigestMismatch.into());
            }
        }
        _ => {
            return Err(inconsistent(
                "prepared exact support count or reference differs",
            ));
        }
    }
    Ok((
        values.clone(),
        HearingSchedulingContext {
            administration_revision: admin.revision,
            administration_digest: admin.values_digest,
            stage_revision: stage.stage_revision(),
            stage: stage.stage(),
            stage_digest: match stage {
                CaseStageEntry::Initial(_) => None,
                CaseStageEntry::Changed(value) => Some(value.values_digest),
            },
        },
    ))
}
