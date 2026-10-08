use super::{authorize, port, preparation, storage, write, PostgresMeasureDecisionStore};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{
    documents::StageSupportReadLimits, identity::Principal, precautionary_measures::*,
    ApplicationError,
};
use domain::cases::CaseId;

fn replay(
    actor: &Principal,
    case: CaseId,
    command: &MeasureDecisionCommand,
    result: &MeasureDecisionStoredOperation,
) -> Result<(), ApplicationError> {
    if result.group.review.case_id != case
        || result.group.review.actor.id != actor.id
        || result.group.review.command != *command
    {
        return Err(MeasureDecisionError::OperationConflict.into());
    }
    Ok(())
}
impl MeasureDecisionStore for PostgresMeasureDecisionStore {
    fn prepare(
        &self,
        actor: &Principal,
        case: CaseId,
        command: &MeasureDecisionCommand,
        limits: &StageSupportReadLimits,
    ) -> Result<MeasureDecisionPreparation, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        authorize(&mut tx, actor, case, true)?;
        let result =
            match storage::operation(&mut tx, case, command.operation_id, self.hasher.as_ref())? {
                Some(result) => {
                    replay(actor, case, command, &result)?;
                    MeasureDecisionPreparation::Replay(Box::new(result))
                }
                None => MeasureDecisionPreparation::Ready(Box::new(preparation::load(
                    &mut tx,
                    case,
                    command,
                    limits,
                    self.hasher.as_ref(),
                )?)),
            };
        // Admission and confirmation may still reject this preparation. No mutation
        // or read event can precede those checks in the submission workflow.
        tx.commit().map_err(port)?;
        Ok(result)
    }
    fn commit(
        &self,
        actor: &Principal,
        case: CaseId,
        prepared: PreparedMeasureDecision,
    ) -> Result<MeasureDecisionStoredOperation, ApplicationError> {
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
            if result.group.review != *prepared.review() {
                return Err(MeasureDecisionError::OperationConflict.into());
            }
            append_transaction(
                &mut tx,
                &actor.email,
                "measure_decision.replay",
                &write::marker(&result.group),
                self.now(Some(result.group.recorded_at))?,
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
        if material != *prepared.material() {
            return Err(MeasureDecisionError::SubmissionMismatch.into());
        }
        let result = prepared.into_operation(self.now(None)?)?;
        proof.validate_forest(case, self.hasher.as_ref(), Some(&result), None)?;
        write::insert(&mut tx, &result, self.hasher.as_ref())?;
        tx.commit().map_err(port)?;
        Ok(result)
    }
}
