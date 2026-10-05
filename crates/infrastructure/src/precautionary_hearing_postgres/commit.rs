use super::{
    authorization::authorize, port, preparation, storage, write, PostgresPrecautionaryHearingStore,
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
    result: &PrecautionaryHearingStoredOperation,
) -> Result<(), ApplicationError> {
    if result.capture.review.case_id != case
        || result.capture.review.actor.id != actor.id
        || result.capture.review.command != *command
    {
        return Err(PrecautionaryHearingError::OperationConflict.into());
    }
    Ok(())
}
impl PrecautionaryHearingStore for PostgresPrecautionaryHearingStore {
    fn prepare(
        &self,
        actor: &Principal,
        case: CaseId,
        command: &PrecautionaryHearingCommand,
        limits: &StageSupportReadLimits,
    ) -> Result<PrecautionaryHearingPreparation, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        authorize(&mut tx, actor, case, true)?;
        let result =
            match storage::operation(&mut tx, case, command.operation_id, self.hasher.as_ref())? {
                Some(result) => {
                    replay(actor, case, command, &result)?;
                    PrecautionaryHearingPreparation::Replay(Box::new(result))
                }
                None => PrecautionaryHearingPreparation::Ready(Box::new(preparation::load(
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
        prepared: PreparedPrecautionaryHearing,
    ) -> Result<PrecautionaryHearingStoredOperation, ApplicationError> {
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
        let material = preparation::load(
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
        write::insert(&mut tx, &result.capture, self.hasher.as_ref())?;
        tx.commit().map_err(port)?;
        Ok(result)
    }
}
