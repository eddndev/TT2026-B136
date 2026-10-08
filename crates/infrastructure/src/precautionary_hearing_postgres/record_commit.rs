use super::{
    authorization::authorize, port, record_preparation, record_storage, write,
    PostgresPrecautionaryHearingStore,
};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{
    documents::StageSupportReadLimits, identity::Principal, precautionary_hearings::*,
    ApplicationError,
};
use domain::cases::CaseId;

fn replay(
    actor: &Principal,
    case: CaseId,
    command: &PrecautionaryHearingCommand,
    result: &PrecautionaryHearingRecordStoredOperation,
) -> Result<(), ApplicationError> {
    if result.capture.review.case_id != case
        || result.capture.review.actor.id != actor.id
        || result.capture.review.command != *command
    {
        return Err(PrecautionaryHearingError::OperationConflict.into());
    }
    Ok(())
}

impl PrecautionaryHearingRecordStore for PostgresPrecautionaryHearingStore {
    fn prepare(
        &self,
        actor: &Principal,
        case: CaseId,
        command: &PrecautionaryHearingCommand,
        limits: &StageSupportReadLimits,
    ) -> Result<PrecautionaryHearingRecordPreparation, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        authorize(&mut tx, actor, case, true)?;
        let result = match record_storage::operation(
            &mut tx,
            case,
            command.operation_id,
            self.hasher.as_ref(),
        )? {
            Some(result) => {
                replay(actor, case, command, &result)?;
                PrecautionaryHearingRecordPreparation::Replay(Box::new(result))
            }
            None => PrecautionaryHearingRecordPreparation::Ready(Box::new(
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
        prepared: PreparedPrecautionaryHearingRecord,
    ) -> Result<PrecautionaryHearingRecordStoredOperation, ApplicationError> {
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
            if result.capture.review != *prepared.review() {
                return Err(PrecautionaryHearingError::OperationConflict.into());
            }
            append_transaction(
                &mut tx,
                &actor.email,
                "precautionary_hearing.replay",
                &write::marker(&result.capture),
                self.now_after(result.capture.recorded_at)?,
            )?;
            tx.commit().map_err(port)?;
            return Ok(result);
        }
        let (material, proof) = record_preparation::load_with_proof(
            &mut tx,
            case,
            command,
            &StageSupportReadLimits::standard(),
            self.hasher.as_ref(),
        )?;
        if material != *prepared.material() {
            return Err(PrecautionaryHearingError::SubmissionMismatch.into());
        }
        let result = prepared.into_operation(self.now()?)?;
        proof.validate_hearing_record_candidate(case, self.hasher.as_ref(), &result)?;
        write::insert(&mut tx, &result.capture, self.hasher.as_ref())?;
        tx.commit().map_err(port)?;
        Ok(result)
    }
}
