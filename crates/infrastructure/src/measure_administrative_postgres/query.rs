use super::{authorize, inconsistent, port, storage, write, PostgresMeasureAdministrativeStore};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{identity::Principal, measure_corrections::*, ApplicationError};
use domain::{cases::CaseId, precautionary_measures::MeasureCorrectionOperationId};

impl MeasureAdministrativeReadStore for PostgresMeasureAdministrativeStore {
    fn list(
        &self,
        actor: &Principal,
        case: CaseId,
        query: MeasureAdministrativeReadQuery,
    ) -> Result<MeasureAdministrativePage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        authorize(&mut tx, actor, case, false)?;
        crate::measure_decision_postgres::audit::inventory_intact(&mut tx)?;
        let rows = tx
            .query(
                "SELECT operation_id FROM case_measure_administrations
             WHERE case_id=$1 AND ($2::uuid IS NULL OR operation_id>$2)
             ORDER BY operation_id LIMIT $3",
                &[
                    &case.as_uuid(),
                    &query.after_operation_id().map(|id| id.as_uuid()),
                    &(i64::from(query.limit()) + 1),
                ],
            )
            .map_err(port)?;
        let has_more = rows.len() > usize::from(query.limit());
        let mut items = Vec::with_capacity(rows.len().min(usize::from(query.limit())));
        for row in rows.iter().take(usize::from(query.limit())) {
            let id = MeasureCorrectionOperationId::from_uuid(row.get(0));
            items.push(
                storage::operation(&mut tx, case, id, self.hasher.as_ref())?
                    .ok_or_else(|| inconsistent("listed administrative operation is absent"))?,
            );
        }
        let next_after_operation_id = if has_more {
            items.last().map(|v| v.origin.operation_id)
        } else {
            None
        };
        let at = self.now(items.iter().map(|v| v.capture.recorded_at).max())?;
        append_transaction(
            &mut tx,
            &actor.email,
            "measure_administrative.list",
            &format!("case:{case}"),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(MeasureAdministrativePage {
            case_id: case,
            items,
            has_more,
            next_after_operation_id,
        })
    }

    fn get_operation(
        &self,
        actor: &Principal,
        case: CaseId,
        operation: MeasureCorrectionOperationId,
    ) -> Result<MeasureAdministrativeStoredOperation, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        authorize(&mut tx, actor, case, false)?;
        let result = storage::operation(&mut tx, case, operation, self.hasher.as_ref())?
            .ok_or(MeasureAdministrativeError::NotFound)?;
        append_transaction(
            &mut tx,
            &actor.email,
            "measure_administrative.operation",
            &write::marker(&result.capture),
            self.now(Some(result.capture.recorded_at))?,
        )?;
        tx.commit().map_err(port)?;
        Ok(result)
    }
}
