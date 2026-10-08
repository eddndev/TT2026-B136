use super::{audit, inconsistent, port, storage};
use crate::measure_decision_postgres::{load_precautionary_history, HistoryReserve, HistoryRoot};
use application::{precautionary_hearings::*, ApplicationError};
use domain::{cases::CaseId, crypto::DocumentHasher, precautionary_hearings::*};
use postgres::Transaction;

pub(crate) fn detail(
    tx: &mut Transaction<'_>,
    case: CaseId,
    id: PrecautionaryHearingId,
    revision: Option<PrecautionaryHearingRevision>,
    hasher: &dyn DocumentHasher,
) -> Result<PrecautionaryHearingRecordStoredOperation, ApplicationError> {
    let selected = storage::selection(tx, case, id, revision)?;
    let loaded = load_precautionary_history(
        tx,
        case,
        &[HistoryRoot::Hearing(selected)],
        HistoryReserve::default(),
        hasher,
    )?;
    loaded.hearing_record_operation(selected, hasher)
}

pub(super) fn operation(
    tx: &mut Transaction<'_>,
    case: CaseId,
    operation: PrecautionaryHearingOperationId,
    hasher: &dyn DocumentHasher,
) -> Result<Option<PrecautionaryHearingRecordStoredOperation>, ApplicationError> {
    let row = tx.query_opt(
        "SELECT case_id,hearing_id,revision FROM case_precautionary_hearing_revisions WHERE operation_id=$1",
        &[&operation.as_uuid()],
    ).map_err(port)?;
    let Some(row) = row else {
        audit::operation_absent(tx, case, operation)?;
        return Ok(None);
    };
    if row.get::<_, uuid::Uuid>("case_id") != case.as_uuid() {
        return Err(PrecautionaryHearingError::OperationConflict.into());
    }
    let id = PrecautionaryHearingId::from_uuid(row.get("hearing_id"));
    let revision = PrecautionaryHearingRevision::new(
        u32::try_from(row.get::<_, i64>("revision")).map_err(inconsistent)?,
    )
    .map_err(inconsistent)?;
    detail(tx, case, id, Some(revision), hasher).map(Some)
}
