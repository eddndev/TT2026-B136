use super::read_support as read;
use application::{
    deadline_reevaluation::{decode_observations, encode_observations},
    deadline_technical::{
        early_technical_deadline_outcome, prepare_technical_deadline_change,
        DeadlineReevaluationCommand, DeadlineReevaluationNoChange, DeadlineReevaluationOutcome,
    },
    deadline_worker::{
        administration_evidence_digest, DeadlineWorkerChecked, DeadlineWorkerOutcome,
    },
    deadlines::DeadlineDetail,
    ApplicationError,
};
use domain::{case_administration::CaseRevision, crypto::DocumentHasher};
use postgres::{Row, Transaction};

pub(super) fn load(
    tx: &mut Transaction<'_>,
    row: &Row,
    base: &DeadlineDetail,
    command: &DeadlineReevaluationCommand,
    tag: &str,
    hasher: &dyn DocumentHasher,
) -> Result<DeadlineWorkerOutcome, ApplicationError> {
    let reason = match tag {
        "retired" => DeadlineReevaluationNoChange::Retired,
        "already_initialized" => DeadlineReevaluationNoChange::AlreadyInitialized,
        "already_observed" => DeadlineReevaluationNoChange::AlreadyObserved,
        "dependency_not_selected" => DeadlineReevaluationNoChange::DependencyNotSelected,
        _ => return Err(read::inconsistent("unknown worker result outcome")),
    };
    if row
        .try_get::<_, Option<i64>>("result_revision")
        .map_err(read::inconsistent)?
        .is_some()
        || row
            .try_get::<_, Option<&[u8]>>("result_submission_digest")
            .map_err(read::inconsistent)?
            .is_some()
        || row
            .try_get::<_, Option<&[u8]>>("result_capture_digest")
            .map_err(read::inconsistent)?
            .is_some()
    {
        return Err(read::inconsistent(
            "no-change result declares a produced revision",
        ));
    }
    read::no_operation_revision(tx, command.operation_id)?;
    let observations: Option<&[u8]> = row
        .try_get("checked_observations_canonical")
        .map_err(read::inconsistent)?;
    let administration_revision: Option<i64> = row
        .try_get("checked_administration_revision")
        .map_err(read::inconsistent)?;
    let administration_digest: Option<&[u8]> = row
        .try_get("checked_administration_evidence_digest")
        .map_err(read::inconsistent)?;
    let checked = if matches!(
        reason,
        DeadlineReevaluationNoChange::Retired | DeadlineReevaluationNoChange::AlreadyInitialized
    ) {
        if observations.is_some()
            || administration_revision.is_some()
            || administration_digest.is_some()
            || early_technical_deadline_outcome(hasher, base, command).map_err(read::stored)?
                != Some(reason)
        {
            return Err(read::inconsistent(
                "base-only worker outcome or evidence differs",
            ));
        }
        None
    } else {
        let bytes =
            observations.ok_or_else(|| read::inconsistent("no-change observations are absent"))?;
        let observations = decode_observations(bytes).map_err(read::inconsistent)?;
        if observations.case_id != base.case_id
            || encode_observations(&observations).map_err(read::inconsistent)? != bytes
        {
            return Err(read::inconsistent(
                "no-change observations are not canonical for this case",
            ));
        }
        let administration_revision = administration_revision
            .map(|value| {
                CaseRevision::new(u32::try_from(value).map_err(read::inconsistent)?)
                    .map_err(read::inconsistent)
            })
            .transpose()?;
        let captured_admin_digest = read::digest(
            administration_digest
                .ok_or_else(|| read::inconsistent("no-change administration digest is absent"))?,
        )?;
        let administration = crate::deadline_postgres::administration::at_revision(
            tx,
            base.case_id,
            administration_revision,
            hasher,
        )
        .map_err(read::stored)?;
        if captured_admin_digest != administration_evidence_digest(hasher, &administration) {
            return Err(read::inconsistent(
                "no-change administration evidence differs",
            ));
        }
        let inputs = crate::deadline_postgres::tracking::observed_inputs(
            tx,
            base,
            &observations,
            &administration,
            hasher,
        )
        .map_err(read::stored)?;
        if !matches!(prepare_technical_deadline_change(hasher, base, command.clone(), inputs).map_err(read::stored)?,
            DeadlineReevaluationOutcome::NoChange(actual) if actual == reason)
        {
            return Err(read::inconsistent(
                "no-change result differs from historical preparation",
            ));
        }
        Some(DeadlineWorkerChecked {
            observations,
            administration_revision,
            administration_evidence_digest: captured_admin_digest,
        })
    };
    Ok(DeadlineWorkerOutcome::NoChange { reason, checked })
}
