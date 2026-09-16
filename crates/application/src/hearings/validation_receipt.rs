use super::*;
use crate::ApplicationError;
use domain::{
    case_administration::CaseStageRevision, case_stages::CaseStage, crypto::DocumentHasher,
};

pub(super) fn inconsistent(message: &str) -> ApplicationError {
    HearingError::StoredInconsistent(message.into()).into()
}

/// Verifies the self-contained receipt. Storage must also resolve its exact sources.
pub fn hearing_receipt_matches(
    hasher: &dyn DocumentHasher,
    detail: &HearingDetail,
) -> Result<(), ApplicationError> {
    let snapshot = &detail.snapshot;
    let receipt = &snapshot.receipt;
    if hearing_values_digest(hasher, &snapshot.values) != snapshot.values_digest {
        return Err(inconsistent("value digest differs from hearing values"));
    }
    let context = snapshot.scheduling_context;
    if context.stage != snapshot.values.kind().required_stage()
        || (context.stage_digest.is_none()
            && (context.stage_revision != CaseStageRevision::FIRST
                || context.stage != CaseStage::Investigation))
        || snapshot.recorded_administration_revision < context.administration_revision
        || (snapshot.recorded_administration_revision == context.administration_revision
            && snapshot.recorded_administration_digest != context.administration_digest)
    {
        return Err(inconsistent("hearing scheduling context is inconsistent"));
    }
    if receipt.action != HearingAction::Cancel
        && snapshot.recorded_administration_revision != context.administration_revision
    {
        return Err(inconsistent(
            "scheduling and recorded administration revisions differ",
        ));
    }
    let reason = snapshot.reason.clone();
    let change = match receipt.action {
        HearingAction::Schedule
            if receipt.expected_revision == 0
                && reason.is_none()
                && snapshot.status == HearingStatus::Scheduled =>
        {
            HearingChange::Schedule {
                context: receipt
                    .expected_context
                    .ok_or_else(|| inconsistent("schedule context is absent"))?,
                values: snapshot.values.clone(),
            }
        }
        HearingAction::Replace if snapshot.status == HearingStatus::Scheduled => {
            HearingChange::Replace {
                expected_revision: HearingRevision::new(receipt.expected_revision)
                    .map_err(|_| inconsistent("replace base is not positive"))?,
                context: receipt
                    .expected_context
                    .ok_or_else(|| inconsistent("replace context is absent"))?,
                values: snapshot.values.clone(),
                reason: reason.ok_or_else(|| inconsistent("replace reason is absent"))?,
            }
        }
        HearingAction::Cancel
            if snapshot.status == HearingStatus::Cancelled
                && receipt.expected_context.is_none() =>
        {
            HearingChange::Cancel {
                expected_revision: HearingRevision::new(receipt.expected_revision)
                    .map_err(|_| inconsistent("cancel base is not positive"))?,
                reason: reason.ok_or_else(|| inconsistent("cancel reason is absent"))?,
            }
        }
        _ => return Err(inconsistent("receipt action disagrees with hearing state")),
    };
    let command = HearingCommand {
        operation_id: receipt.operation_id,
        hearing_id: snapshot.id,
        change,
    };
    if command
        .result_revision()
        .map_err(|_| inconsistent("receipt revision is exhausted"))?
        != snapshot.revision
    {
        return Err(inconsistent("receipt does not produce the stored revision"));
    }
    if command.expected_context().is_some_and(|expected| {
        expected.case_revision != context.administration_revision
            || expected.stage_revision != context.stage_revision
    }) {
        return Err(inconsistent(
            "receipt context differs from scheduling context",
        ));
    }
    if hearing_submission_digest(
        hasher,
        snapshot.recorded_by.id,
        snapshot.case_id,
        &command,
        snapshot.values_digest,
    ) != receipt.submission_digest
    {
        return Err(inconsistent(
            "receipt digest differs from reconstructed command",
        ));
    }
    validate_participant_projection(snapshot.case_id, &snapshot.values, &detail.participants)?;
    match (snapshot.values.conviction_basis(), &detail.support) {
        (None, None) => {}
        (Some(basis), Some(support))
            if basis.support().reference() == support.reference
                && basis.support().digest() == support.digest => {}
        _ => {
            return Err(inconsistent(
                "historical support differs from declared basis",
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_participant_projection(
    case_id: domain::cases::CaseId,
    values: &HearingValues,
    participants: &[HearingParticipantSnapshot],
) -> Result<(), ApplicationError> {
    if participants.len() != values.participants().len() {
        return Err(inconsistent("participant projection count differs"));
    }
    for (reference, participant) in values.participants().iter().zip(participants) {
        let overview = &participant.overview;
        if overview.case_id != case_id
            || overview.id != reference.id()
            || overview.revision != reference.revision()
            || overview.kind.is_some() != overview.subject.is_some()
            || overview.display_name.trim().is_empty()
            || overview.procedural_role.trim().is_empty()
        {
            return Err(inconsistent(
                "participant projection scope or exact reference differs",
            ));
        }
    }
    Ok(())
}
