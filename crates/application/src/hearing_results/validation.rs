use super::validation_receipt::{inconsistent, validate_attendees};
use super::*;
use crate::{
    cases::case_administration_digest, hearings::hearing_receipt_matches, ApplicationError,
};
use domain::{
    case_administration::CaseAdministrativeStatus, cases::CaseId, crypto::DocumentHasher,
};

pub(super) fn validate_preparation(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    command: &HearingResultCommand,
    prep: &mut HearingResultPreparation,
) -> Result<HearingResultValues, ApplicationError> {
    let admin = prep
        .administration
        .snapshot()
        .ok_or_else(|| inconsistent("exact hearing has no recorded administration"))?;
    if prep.case_id != case_id
        || admin.case_id != case_id
        || case_administration_digest(hasher, &admin.values) != admin.values_digest
    {
        return Err(inconsistent(
            "prepared administration scope or digest differs",
        ));
    }
    if admin.values.status() != CaseAdministrativeStatus::Active {
        return Err(ApplicationError::CaseClosed);
    }
    if prep.anchor.snapshot.case_id != case_id || prep.anchor.snapshot.id != command.hearing_id {
        return Err(inconsistent("prepared anchor belongs to another hearing"));
    }
    hearing_receipt_matches(hasher, &prep.anchor)?;
    let anchor = &prep.anchor.snapshot;
    if admin.revision < anchor.recorded_administration_revision
        || (admin.revision == anchor.recorded_administration_revision
            && admin.values_digest != anchor.recorded_administration_digest)
    {
        return Err(inconsistent(
            "observed administration predates or contradicts anchor",
        ));
    }
    let resolved_anchor = HearingResultAnchorSnapshot::from(&prep.anchor).reference;
    let resolved_continuation = prep
        .continuation
        .as_ref()
        .map(HearingResultContinuationSnapshot::from)
        .map(|p| p.reference);
    if let Some(previous) = &prep.continuation {
        if previous.case_id != case_id || previous.id == command.result_id {
            return Err(HearingResultError::InvalidReference.into());
        }
        hearing_result_snapshot_receipt_matches(hasher, previous)?;
        if admin.revision < previous.recorded_administration_revision
            || (admin.revision == previous.recorded_administration_revision
                && admin.values_digest != previous.recorded_administration_digest)
        {
            return Err(inconsistent(
                "observed administration predates or contradicts continuation",
            ));
        }
    }
    let base = prep.base.as_ref();
    match (&command.change, base) {
        (HearingResultChange::Record { .. }, Some(_)) => {
            return Err(HearingResultError::RevisionConflict.into())
        }
        (HearingResultChange::Correct { .. } | HearingResultChange::Withdraw { .. }, None) => {
            return Err(HearingResultError::NotFound.into())
        }
        _ => {}
    }
    if let Some(base) = base {
        let s = &base.snapshot;
        if s.case_id != case_id || s.hearing_id != command.hearing_id || s.id != command.result_id {
            return Err(inconsistent("prepared base belongs to another session"));
        }
        if s.revision.get() != command.expected_revision() {
            return Err(HearingResultError::RevisionConflict.into());
        }
        if s.status == HearingResultStatus::Withdrawn {
            return Err(HearingResultError::AlreadyWithdrawn.into());
        }
        hearing_result_receipt_matches(hasher, base)?;
        if s.receipt.operation_id == command.operation_id {
            return Err(HearingResultError::OperationConflict.into());
        }
        if admin.revision < s.recorded_administration_revision
            || (admin.revision == s.recorded_administration_revision
                && admin.values_digest != s.recorded_administration_digest)
        {
            return Err(inconsistent(
                "observed administration predates or contradicts base",
            ));
        }
        if s.anchor != resolved_anchor
            || s.continuation != resolved_continuation
            || base.anchor != HearingResultAnchorSnapshot::from(&prep.anchor)
            || base.continuation
                != prep
                    .continuation
                    .as_ref()
                    .map(HearingResultContinuationSnapshot::from)
        {
            return Err(inconsistent("fixed source of existing session changed"));
        }
    }
    let values = match &command.change {
        HearingResultChange::Record {
            anchor_revision,
            continuation,
            values,
        } => {
            if *anchor_revision != resolved_anchor.revision
                || continuation.map(|v| (v.id(), v.revision()))
                    != resolved_continuation.map(|v| (v.result_id, v.revision))
            {
                return Err(HearingResultError::InvalidReference.into());
            }
            values.clone()
        }
        HearingResultChange::Correct { values, .. } => values.clone(),
        HearingResultChange::Withdraw { .. } => {
            let base = base.ok_or_else(|| inconsistent("withdrawal lacks a base"))?;
            if prep.attendees != base.attendees || !prep.records.is_empty() {
                return Err(inconsistent(
                    "withdrawal does not preserve historical projections",
                ));
            }
            return Ok(base.snapshot.values.clone());
        }
    };
    prep.attendees
        .sort_by_key(|p| p.participant.overview.id.as_uuid());
    validate_attendees(case_id, &values, &prep.attendees)?;
    if let Some(base) = &prep.base {
        for attendee in &prep.attendees {
            let reference = &attendee.participant.overview;
            let retained = base.attendees.iter().find(|old| {
                old.participant.overview.id == reference.id
                    && old.participant.overview.revision == reference.revision
            });
            if retained.is_some_and(|old| old != attendee) {
                return Err(inconsistent("retained exact attendee projection changed"));
            }
        }
    }

    match (values.provenance().support(), prep.records.as_slice()) {
        (None, []) => {}
        (Some(support), [record])
            if record.id == support.reference().id
                && record.version == support.reference().version =>
        {
            if record.digest != support.digest() {
                return Err(HearingResultError::SupportDigestMismatch.into());
            }
        }
        _ => {
            return Err(inconsistent(
                "prepared exact support count or reference differs",
            ))
        }
    }
    Ok(values)
}
