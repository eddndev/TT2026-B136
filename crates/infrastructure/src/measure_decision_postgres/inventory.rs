use super::{audit, inconsistent, port};
use application::ApplicationError;
use domain::{cases::CaseId, precautionary_measures::MeasureDecisionOperationId};
use postgres::Client;
pub(crate) fn validate_inventory(client: &mut Client) -> Result<(), ApplicationError> {
    let mut tx = crate::audit_postgres::begin_audited(client)?;
    audit::inventory_intact(&mut tx)?;
    let mut after: Option<uuid::Uuid> = None;
    loop {
        let rows=tx.query("SELECT d.decision_id,d.operation_id,d.case_id,o.family FROM case_measure_decisions d
            JOIN case_measure_operations o ON o.operation_id=d.operation_id AND o.case_id=d.case_id
            WHERE ($1::uuid IS NULL OR d.decision_id>$1) ORDER BY d.decision_id LIMIT 32",&[&after]).map_err(port)?;
        if rows.is_empty() {
            break;
        }
        for row in rows {
            let id: uuid::Uuid = row.get("decision_id");
            let operation = MeasureDecisionOperationId::from_uuid(row.get("operation_id"));
            let loaded = super::load_precautionary_history(
                &mut tx,
                CaseId::from_uuid(row.get("case_id")),
                &[super::HistoryRoot::Decision(operation)],
                super::HistoryReserve::default(),
                &crate::RingSha256Hasher,
            )?;
            match row.get::<_, String>("family").as_str() {
                "g1" => {
                    loaded.into_operation(operation.as_uuid())?;
                }
                "g2" => {
                    loaded.into_record_operation(operation)?;
                }
                _ => return Err(inconsistent("judicial startup owner has another family")),
            }
            after = Some(id);
        }
    }
    let mut after: Option<uuid::Uuid> = None;
    loop {
        let rows = tx
            .query(
                "SELECT operation_id,case_id FROM case_measure_administrations
            WHERE ($1::uuid IS NULL OR operation_id>$1) ORDER BY operation_id LIMIT 8",
                &[&after],
            )
            .map_err(port)?;
        if rows.is_empty() {
            break;
        }
        for row in rows {
            let id: uuid::Uuid = row.get("operation_id");
            let case = CaseId::from_uuid(row.get("case_id"));
            let op = domain::precautionary_measures::MeasureCorrectionOperationId::from_uuid(id);
            super::load_precautionary_history(
                &mut tx,
                case,
                &[super::HistoryRoot::Administrative(op)],
                super::HistoryReserve::default(),
                &crate::RingSha256Hasher,
            )?
            .into_administrative_operation(op)?;
            after = Some(id);
        }
    }
    tx.commit().map_err(port)
}
