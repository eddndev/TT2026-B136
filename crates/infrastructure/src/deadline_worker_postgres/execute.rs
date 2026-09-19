use super::{attempts_read, inconsistent, port, results, results_write, selection};
use application::{
    deadline_observations::build_deadline_observations, deadline_reevaluation::PredecessorReceipt,
    deadline_technical::*, deadline_worker::*, deadlines::*, ApplicationError,
};
use domain::crypto::DocumentHasher;
use postgres::{Client, Transaction};
use time::OffsetDateTime;

#[derive(Default)]
pub(super) struct FailureContext {
    pub target: Option<selection::Target>,
    pub checked_base: Option<DeadlineWorkerBase>,
    pub verifying_job: bool,
}

pub(super) fn run(
    client: &mut Client,
    hasher: &dyn DocumentHasher,
    at: OffsetDateTime,
    context: &mut FailureContext,
) -> Result<DeadlineWorkerRun, ApplicationError> {
    let mut tx = crate::audit_postgres::begin_audited(client)?;
    let Some(target) = selection::next(&mut tx, at)? else {
        tx.commit().map_err(port)?;
        return Ok(DeadlineWorkerRun::Idle);
    };
    context.target = Some(target);
    context.verifying_job = true;
    let job = crate::deadline_worker_provenance::load_job(&mut tx, target.id, hasher)?;
    context.verifying_job = false;
    if job.operation_id != target.operation_id {
        return Err(inconsistent("selected worker operation changed"));
    }
    let base = crate::deadline_postgres::storage::detail(
        &mut tx,
        job.case_id,
        job.deadline_id,
        None,
        hasher,
    )?;
    let saved_base = DeadlineWorkerBase {
        revision: base.revision,
        receipt: PredecessorReceipt {
            submission_digest: base.receipt.submission_digest,
            capture_digest: base.receipt.capture_digest,
        },
    };
    context.checked_base = Some(saved_base);
    // Authenticate the selected retry ledger rather than trusting scheduling projections.
    attempts_read::latest(&mut tx, job.id, hasher)?;
    let command = DeadlineReevaluationCommand {
        operation_id: job.operation_id,
        cause: job.cause,
    };
    let outcome = if let Some(reason) = early_technical_deadline_outcome(hasher, &base, &command)? {
        DeadlineWorkerOutcome::NoChange {
            reason,
            checked: None,
        }
    } else {
        prepare_and_write(&mut tx, hasher, &base, command.clone(), at)?
    };
    let expected = DeadlineWorkerResult {
        job_id: job.id,
        case_id: job.case_id,
        deadline_id: job.deadline_id,
        command,
        base: saved_base,
        outcome,
        completed_at: at,
    };
    results_write::insert(&mut tx, &expected)?;
    let actual = results::load(&mut tx, job.id, hasher)?
        .ok_or_else(|| inconsistent("inserted worker result disappeared"))?;
    if actual != expected {
        return Err(inconsistent(
            "stored worker result differs from its preparation",
        ));
    }
    results_write::audit(&mut tx, &actual)?;
    tx.commit().map_err(port)?;
    Ok(DeadlineWorkerRun::Completed(Box::new(actual)))
}

fn prepare_and_write(
    tx: &mut Transaction<'_>,
    hasher: &dyn DocumentHasher,
    base: &DeadlineDetail,
    command: DeadlineReevaluationCommand,
    at: OffsetDateTime,
) -> Result<DeadlineWorkerOutcome, ApplicationError> {
    let definition = &base.definition;
    let inputs = DeadlineReevaluationInputs {
        profile_head: crate::deadline_profile_postgres::storage::detail(
            tx,
            definition.profile.id,
            None,
            hasher,
        )?,
        material: crate::deadline_input_postgres::load_material(
            tx,
            &definition.input.selection,
            definition.input.calendar,
            hasher,
        )?,
        notification_parent_head:
            crate::deadline_postgres::preparation::notification_parent_for_definition(
                tx,
                base.case_id,
                definition,
                hasher,
            )?,
    };
    match prepare_technical_deadline_change(hasher, base, command, inputs.clone())? {
        DeadlineReevaluationOutcome::Revision(prepared) => {
            let next = prepared.record(at);
            deadline_successor_matches(hasher, base, &next)?;
            crate::deadline_postgres::write::insert(tx, &next, hasher)?;
            Ok(DeadlineWorkerOutcome::Revision {
                revision: next.revision,
                receipt: Box::new(next.receipt),
            })
        }
        DeadlineReevaluationOutcome::NoChange(reason) => {
            let observations = build_deadline_observations(
                hasher,
                base.case_id,
                &inputs.profile_head,
                &inputs.material,
                inputs.notification_parent_head.as_ref(),
            )?;
            let administration = &inputs.material.administration;
            Ok(DeadlineWorkerOutcome::NoChange {
                reason,
                checked: Some(DeadlineWorkerChecked {
                    observations,
                    administration_revision: administration.revision(),
                    administration_evidence_digest: administration_evidence_digest(
                        hasher,
                        administration,
                    ),
                }),
            })
        }
    }
}
