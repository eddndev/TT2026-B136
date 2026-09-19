use super::{inconsistent, port};
use application::{
    deadline_reevaluation::{encode_observations, TechnicalCause},
    deadline_technical::DeadlineReevaluationNoChange,
    deadline_worker::*,
    ApplicationError,
};
use postgres::Transaction;
use serde_json::{json, Value};

pub(super) fn insert(
    tx: &mut Transaction<'_>,
    value: &DeadlineWorkerResult,
) -> Result<(), ApplicationError> {
    let (kind, revision, submission, capture, checked) = match &value.outcome {
        DeadlineWorkerOutcome::Revision { revision, receipt } => (
            "revision",
            Some(i64::from(revision.get())),
            Some(receipt.submission_digest.as_bytes().as_slice()),
            Some(receipt.capture_digest.as_bytes().as_slice()),
            None,
        ),
        DeadlineWorkerOutcome::NoChange { reason, checked } => {
            (reason_text(*reason), None, None, None, checked.as_ref())
        }
    };
    let observations = checked
        .map(|c| encode_observations(&c.observations).map_err(inconsistent))
        .transpose()?;
    let administration = checked
        .and_then(|c| c.administration_revision)
        .map(|r| i64::from(r.get()));
    let administration_digest =
        checked.map(|c| c.administration_evidence_digest.as_bytes().as_slice());
    tx.execute(
        "INSERT INTO deadline_reevaluation_results(
        job_id,base_revision,base_submission_digest,base_capture_digest,outcome,
        result_revision,result_submission_digest,result_capture_digest,
        checked_observations_canonical,checked_administration_revision,
        checked_administration_evidence_digest,completed_at_seconds,completed_at_nanoseconds)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
        &[
            &value.job_id,
            &i64::from(value.base.revision.get()),
            &value.base.receipt.submission_digest.as_bytes().as_slice(),
            &value.base.receipt.capture_digest.as_bytes().as_slice(),
            &kind,
            &revision,
            &submission,
            &capture,
            &observations,
            &administration,
            &administration_digest,
            &value.completed_at.unix_timestamp(),
            &(value.completed_at.nanosecond() as i32),
        ],
    )
    .map_err(port)?;
    Ok(())
}

pub(super) fn audit(
    tx: &mut Transaction<'_>,
    value: &DeadlineWorkerResult,
) -> Result<(), ApplicationError> {
    let (action, outcome) = match &value.outcome {
        DeadlineWorkerOutcome::Revision { revision, receipt } => (
            "deadline.reevaluated",
            json!({
                "kind": "revision", "revision": revision.get(),
                "submission": receipt.submission_digest.to_hex(), "capture": receipt.capture_digest.to_hex(),
            }),
        ),
        DeadlineWorkerOutcome::NoChange { reason, checked } => {
            let checked = checked
                .as_ref()
                .map(|c| {
                    let bytes = encode_observations(&c.observations).map_err(inconsistent)?;
                    Ok::<_, ApplicationError>(json!({
                        "observations": bytes,
                        "administration_revision": c.administration_revision.map(|r| r.get()),
                        "administration_evidence": c.administration_evidence_digest.to_hex(),
                    }))
                })
                .transpose()?;
            (
                "deadline.reevaluation_no_change",
                json!({"kind": reason_text(*reason), "checked": checked}),
            )
        }
    };
    let resource = json!({
        "case": value.case_id.to_string(), "deadline": value.deadline_id.to_string(),
        "job": value.job_id.to_string(), "operation": value.command.operation_id.to_string(),
        "cause": cause(value.command.cause), "base": value.base.revision.get(),
        "base_submission": value.base.receipt.submission_digest.to_hex(),
        "base_capture": value.base.receipt.capture_digest.to_hex(), "outcome": outcome,
        "completed_at_seconds": value.completed_at.unix_timestamp(),
        "completed_at_nanoseconds": value.completed_at.nanosecond(),
    });
    crate::audit_postgres::append_transaction(
        tx,
        "deadline_reevaluator",
        action,
        &resource.to_string(),
        value.completed_at,
    )?;
    Ok(())
}

fn cause(cause: TechnicalCause) -> Value {
    match cause {
        TechnicalCause::LegacyBootstrap { policy_version, .. } => {
            json!({"kind": "legacy_bootstrap", "policy": policy_version})
        }
        TechnicalCause::SourceEvent { event, .. } => json!({
            "kind": "source_event", "sequence": event.sequence, "family": format!("{:?}", event.family),
            "source": event.source_id.to_string(), "revision": event.revision,
            "case": event.case_id.map(|id| id.to_string()), "hearing": event.hearing_id.map(|id| id.to_string()),
            "operation": event.operation_id.to_string(),
        }),
    }
}

fn reason_text(reason: DeadlineReevaluationNoChange) -> &'static str {
    match reason {
        DeadlineReevaluationNoChange::Retired => "retired",
        DeadlineReevaluationNoChange::AlreadyObserved => "already_observed",
        DeadlineReevaluationNoChange::DependencyNotSelected => "dependency_not_selected",
        DeadlineReevaluationNoChange::AlreadyInitialized => "already_initialized",
    }
}
