use super::{
    authorize, port, record_preparation, record_storage, record_write, PostgresMeasureDecisionStore,
};
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
    result: &MeasureDecisionRecordReceipt,
) -> Result<(), ApplicationError> {
    if result.case_id() != case || result.actor().id != actor.id || result.command() != command {
        return Err(MeasureDecisionError::OperationConflict.into());
    }
    Ok(())
}

impl MeasureDecisionRecordStore for PostgresMeasureDecisionStore {
    fn prepare(
        &self,
        actor: &Principal,
        case: CaseId,
        command: &MeasureDecisionCommand,
        limits: &StageSupportReadLimits,
    ) -> Result<MeasureDecisionRecordPreparation, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        authorize(&mut tx, actor, case, true)?;
        let result =
            match record_storage::operation(
                &mut tx,
                case,
                command.operation_id,
                self.hasher.as_ref(),
            )? {
                Some(result) => {
                    replay(actor, case, command, &result)?;
                    MeasureDecisionRecordPreparation::Replay(Box::new(result))
                }
                None => MeasureDecisionRecordPreparation::Ready(Box::new(
                    record_preparation::load(&mut tx, case, command, limits, self.hasher.as_ref())?,
                )),
            };
        tx.commit().map_err(port)?;
        Ok(result)
    }

    fn commit(
        &self,
        actor: &Principal,
        case: CaseId,
        prepared: PreparedMeasureDecisionRecord,
    ) -> Result<MeasureDecisionRecordStoredOperation, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        authorize(&mut tx, actor, case, true)?;
        if prepared.actor() != actor || prepared.review().case_id != case {
            return Err(ApplicationError::InvalidSession);
        }
        let command = &prepared.review().command;
        if let Some(result) =
            record_storage::operation(&mut tx, case, command.operation_id, self.hasher.as_ref())?
        {
            replay(actor, case, command, &result)?;
            let MeasureDecisionRecordReceipt::V2(result) = result else {
                return Err(MeasureDecisionError::OperationConflict.into());
            };
            if result.group.review != *prepared.review() {
                return Err(MeasureDecisionError::OperationConflict.into());
            }
            append_transaction(
                &mut tx,
                &actor.email,
                "measure_decision.replay",
                &record_write::marker(&result.group),
                self.now(Some(result.group.recorded_at))?,
            )?;
            tx.commit().map_err(port)?;
            return Ok(*result);
        }
        let (material, proof) = record_preparation::load_with_proof(
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
        proof.validate_record_candidate(case, self.hasher.as_ref(), &result)?;
        record_write::insert(&mut tx, &result, self.hasher.as_ref())?;
        tx.commit().map_err(port)?;
        Ok(result)
    }
}
