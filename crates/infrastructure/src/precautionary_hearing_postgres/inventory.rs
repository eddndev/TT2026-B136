use super::{inconsistent, port, record_storage};
use application::ApplicationError;
use domain::{cases::CaseId, precautionary_hearings::PrecautionaryHearingId};
use postgres::Client;
pub(crate) fn validate_inventory(client: &mut Client) -> Result<(), ApplicationError> {
    let mut tx = crate::audit_postgres::begin_audited(client)?;
    let invalid:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM case_precautionary_hearing_revisions r LEFT JOIN case_precautionary_hearings h ON h.id=r.hearing_id AND h.case_id=r.case_id WHERE h.id IS NULL)
 OR EXISTS(SELECT 1 FROM audit_events a LEFT JOIN case_precautionary_hearing_revisions r ON r.audit_sequence=a.sequence WHERE a.action IN ('precautionary_hearing.schedule','precautionary_hearing.replace','precautionary_hearing.cancel') AND r.audit_sequence IS NULL)",&[]).map_err(port)?.get(0);
    if invalid {
        return Err(inconsistent("orphan precautionary revision or audit event"));
    }
    let mut after: Option<uuid::Uuid> = None;
    loop {
        let rows=tx.query("SELECT id,case_id FROM case_precautionary_hearings WHERE ($1::uuid IS NULL OR id>$1) ORDER BY id LIMIT 32",&[&after]).map_err(port)?;
        if rows.is_empty() {
            break;
        }
        for row in rows {
            let id: uuid::Uuid = row.get("id");
            record_storage::detail(
                &mut tx,
                CaseId::from_uuid(row.get("case_id")),
                PrecautionaryHearingId::from_uuid(id),
                None,
                &crate::RingSha256Hasher,
            )?;
            after = Some(id);
        }
    }
    tx.commit().map_err(port)
}
