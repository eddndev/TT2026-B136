use super::{audit, port, storage};
use application::ApplicationError;
use domain::{cases::CaseId, precautionary_measures::MeasureDecisionId};
use postgres::Client;
pub(crate) fn validate_inventory(client: &mut Client) -> Result<(), ApplicationError> {
    let mut tx = crate::audit_postgres::begin_audited(client)?;
    audit::inventory_intact(&mut tx)?;
    let mut after: Option<uuid::Uuid> = None;
    loop {
        let rows=tx.query("SELECT decision_id,case_id FROM case_measure_decisions WHERE ($1::uuid IS NULL OR decision_id>$1) ORDER BY decision_id LIMIT 32",&[&after]).map_err(port)?;
        if rows.is_empty() {
            break;
        }
        for row in rows {
            let id: uuid::Uuid = row.get("decision_id");
            storage::detail(
                &mut tx,
                CaseId::from_uuid(row.get("case_id")),
                MeasureDecisionId::from_uuid(id),
                &crate::RingSha256Hasher,
            )?;
            after = Some(id);
        }
    }
    tx.commit().map_err(port)
}
