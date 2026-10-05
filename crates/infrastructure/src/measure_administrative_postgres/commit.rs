use super::{authorize, port, preparation, storage, write, PostgresMeasureAdministrativeStore};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{
    documents::StageSupportReadLimits, identity::Principal, measure_corrections::*,
    ApplicationError,
};
use domain::cases::CaseId;

fn replay(
    actor: &Principal,
    case: CaseId,
    command: &MeasureAdministrativeCommand,
    result: &MeasureAdministrativeStoredOperation,
) -> Result<(), ApplicationError> {
    if result.capture.review.case_id != case
        || result.capture.review.actor.id != actor.id
        || result.capture.review.command != *command
    {
        return Err(MeasureAdministrativeError::OperationConflict.into());
    }
    Ok(())
}

impl MeasureAdministrativeStore for PostgresMeasureAdministrativeStore {
    fn prepare(
        &self,
        actor: &Principal,
        case: CaseId,
        command: &MeasureAdministrativeCommand,
        limits: &StageSupportReadLimits,
    ) -> Result<MeasureAdministrativePreparation, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        authorize(&mut tx, actor, case, true)?;
        let result =
            match storage::operation(&mut tx, case, command.operation_id, self.hasher.as_ref())? {
                Some(result) => {
                    replay(actor, case, command, &result)?;
                    MeasureAdministrativePreparation::Replay(Box::new(result))
                }
                None => MeasureAdministrativePreparation::Ready(Box::new(preparation::load(
                    &mut tx,
                    case,
                    command,
                    limits,
                    self.hasher.as_ref(),
                )?)),
            };
        tx.commit().map_err(port)?;
        Ok(result)
    }

    fn commit(
        &self,
        actor: &Principal,
        case: CaseId,
        prepared: PreparedMeasureAdministrative,
    ) -> Result<MeasureAdministrativeStoredOperation, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        authorize(&mut tx, actor, case, true)?;
        if prepared.actor() != actor || prepared.review().case_id != case {
            return Err(ApplicationError::InvalidSession);
        }
        let command = &prepared.review().command;
        if let Some(result) =
            storage::operation(&mut tx, case, command.operation_id, self.hasher.as_ref())?
        {
            replay(actor, case, command, &result)?;
            if result.capture.review != *prepared.review() {
                return Err(MeasureAdministrativeError::OperationConflict.into());
            }
            append_transaction(
                &mut tx,
                &actor.email,
                "measure_administrative.replay",
                &write::marker(&result.capture),
                self.now(Some(result.capture.recorded_at))?,
            )?;
            tx.commit().map_err(port)?;
            return Ok(result);
        }
        let (material, proof) = preparation::load_with_proof(
            &mut tx,
            case,
            command,
            &StageSupportReadLimits::standard(),
            self.hasher.as_ref(),
        )?;
        if material.context != prepared.material().context
            || material.target_head != prepared.material().target_head
            || material.support_record != prepared.material().support_record
        {
            return Err(MeasureAdministrativeError::SubmissionMismatch.into());
        }
        let checked = inspect_measure_administrative_dependencies(
            self.hasher.as_ref(),
            case,
            command.target,
            &material.dependency_inventory,
        )?;
        if !checked.dependants().is_empty() {
            return Err(MeasureAdministrativeError::KnownDependants.into());
        }
        let history = proof.record_subclosure(&[command.target])?;
        let reviewed = prepare_measure_administrative_record_with_decision_history(
            self.hasher.as_ref(),
            actor,
            case,
            command.clone(),
            material.context,
            &history,
        )?;
        if reviewed.review() != prepared.review() {
            return Err(MeasureAdministrativeError::SubmissionMismatch.into());
        }
        let result = prepared.into_operation(self.now(None)?)?;
        if !history_matches(&history, &result.record_history) {
            return Err(MeasureAdministrativeError::SubmissionMismatch.into());
        }
        proof.validate_administrative_candidate(case, self.hasher.as_ref(), &result)?;
        write::insert(&mut tx, &result, self.hasher.as_ref())?;
        tx.commit().map_err(port)?;
        Ok(result)
    }
}

fn history_matches(
    a: &application::precautionary_measures::MeasureDecisionRecordHistoryEvidence,
    b: &application::precautionary_measures::MeasureDecisionRecordHistoryEvidence,
) -> bool {
    let mut ga: Vec<_> = a.records.judicial.groups.iter().collect();
    let mut gb: Vec<_> = b.records.judicial.groups.iter().collect();
    ga.sort_by_key(|g| g.origin.operation_id.as_uuid());
    gb.sort_by_key(|g| g.origin.operation_id.as_uuid());
    let mut aa: Vec<_> = a.records.administrative.iter().collect();
    let mut ab: Vec<_> = b.records.administrative.iter().collect();
    aa.sort_by_key(|g| g.origin.operation_id.as_uuid());
    ab.sort_by_key(|g| g.origin.operation_id.as_uuid());
    let mut da: Vec<_> = a.decisions.iter().collect();
    let mut db: Vec<_> = b.decisions.iter().collect();
    da.sort_by_key(|g| g.origin.operation_id.as_uuid());
    db.sort_by_key(|g| g.origin.operation_id.as_uuid());
    ga == gb && aa == ab && da == db
}
