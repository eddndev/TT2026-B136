use super::{inconsistent, port, sources, storage};
use application::{
    case_stages::StageSupportRef, documents::StageSupportReadLimits, hearing_results::*,
    ApplicationError,
};
use domain::{
    case_administration::CaseAdministrativeStatus, cases::CaseId, crypto::DocumentHasher,
};
use postgres::Transaction;

pub(super) fn load(
    tx: &mut Transaction<'_>,
    case: CaseId,
    command: &HearingResultCommand,
    limits: &StageSupportReadLimits,
    hasher: &dyn DocumentHasher,
) -> Result<HearingResultPreparation, ApplicationError> {
    command.result_revision()?;
    let used: bool = tx
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM case_hearing_result_revisions WHERE operation_id=$1)",
            &[&command.operation_id.as_uuid()],
        )
        .map_err(port)?
        .get(0);
    if used {
        return Err(HearingResultError::OperationConflict.into());
    }
    let base = match storage::detail(
        tx,
        case,
        command.hearing_id,
        command.result_id,
        None,
        hasher,
    ) {
        Ok(value) => Some(value),
        Err(ApplicationError::HearingResult(HearingResultError::NotFound)) => None,
        Err(error) => return Err(error),
    };
    match (&command.change, &base) {
        (HearingResultChange::Record { .. }, Some(_)) => {
            return Err(HearingResultError::RevisionConflict.into())
        }
        (HearingResultChange::Correct { .. } | HearingResultChange::Withdraw { .. }, None) => {
            return Err(HearingResultError::NotFound.into())
        }
        _ => {}
    }
    if let Some(base) = &base {
        if base.snapshot.revision.get() != command.expected_revision() {
            return Err(HearingResultError::RevisionConflict.into());
        }
        if base.snapshot.status != HearingResultStatus::Recorded {
            return Err(HearingResultError::AlreadyWithdrawn.into());
        }
    }
    let administration = crate::cases::storage::detail(tx, case, hasher)?.administration;
    if administration.values().status() != CaseAdministrativeStatus::Active {
        return Err(ApplicationError::CaseClosed);
    }
    let (anchor_revision, continuation) = match &command.change {
        HearingResultChange::Record {
            anchor_revision,
            continuation,
            ..
        } => (*anchor_revision, *continuation),
        _ => {
            let snapshot = &base
                .as_ref()
                .ok_or_else(|| inconsistent("result mutation has no base"))?
                .snapshot;
            (
                snapshot.anchor.revision,
                snapshot
                    .continuation
                    .map(|c| HearingResultContinuationRef::new(c.result_id, c.revision)),
            )
        }
    };
    if continuation.is_some_and(|c| c.id() == command.result_id) {
        return Err(HearingResultError::InvalidReference.into());
    }
    let anchor = sources::anchor(tx, case, command.hearing_id, anchor_revision, hasher)?;
    let continuation = continuation
        .map(|reference| sources::continuation(tx, case, reference, hasher))
        .transpose()?;
    let (attendees, records) = match &command.change {
        HearingResultChange::Withdraw { .. } => (
            base.as_ref()
                .ok_or_else(|| inconsistent("withdrawal has no base"))?
                .attendees
                .clone(),
            Vec::new(),
        ),
        HearingResultChange::Record { values, .. }
        | HearingResultChange::Correct { values, .. } => {
            let attendees = sources::attendees(tx, case, values, hasher)?;
            let records = values
                .provenance()
                .support()
                .map(|support| {
                    crate::case_stages::documents::load(
                        tx,
                        case,
                        StageSupportRef::new(support.reference(), support.digest()),
                        limits,
                    )
                    .map_err(|e| match e {
                        ApplicationError::DocumentNotFound(_) => {
                            HearingResultError::ReferenceNotFound.into()
                        }
                        other => other,
                    })
                })
                .transpose()?
                .into_iter()
                .collect();
            (attendees, records)
        }
    };
    Ok(HearingResultPreparation {
        case_id: case,
        base,
        administration,
        anchor,
        continuation,
        attendees,
        records,
    })
}
