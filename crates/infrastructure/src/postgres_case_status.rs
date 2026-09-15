use application::ApplicationError;
use domain::cases::CaseId;
use postgres::Transaction;

/// Called after authorization and item resolution within the committing transaction.
pub(crate) fn require_active(
    tx: &mut Transaction<'_>,
    case: CaseId,
) -> Result<(), ApplicationError> {
    let closed: bool = tx.query_one("SELECT COALESCE((SELECT administrative_status='closed' FROM case_administration_revisions WHERE case_id=$1 ORDER BY revision DESC LIMIT 1),FALSE)", &[&case.as_uuid()])
        .map_err(|e|ApplicationError::Port(format!("case status database: {e}")))?.get(0);
    if closed {
        Err(ApplicationError::CaseClosed)
    } else {
        Ok(())
    }
}
