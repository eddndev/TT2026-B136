use super::{
    audit, authorize, port, record_storage as storage, record_write, write,
    PostgresMeasureDecisionStore,
};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{identity::Principal, precautionary_measures::*, ApplicationError};
use domain::{cases::CaseId, precautionary_measures::*};
impl MeasureDecisionRecordReadStore for PostgresMeasureDecisionStore {
    fn list(
        &self,
        actor: &Principal,
        case: CaseId,
        query: MeasureDecisionReadQuery,
    ) -> Result<MeasureDecisionRecordPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        authorize(&mut tx, actor, case, false)?;
        audit::inventory_intact(&mut tx)?;
        let rows=tx.query("SELECT decision_id FROM case_measure_decisions WHERE case_id=$1 AND ($2::uuid IS NULL OR decision_id>$2) ORDER BY decision_id LIMIT $3",&[&case.as_uuid(),&query.after_id().map(|v|v.as_uuid()),&(i64::from(query.limit())+1)]).map_err(port)?;
        let has_more = rows.len() > usize::from(query.limit());
        let mut items = Vec::with_capacity(rows.len().min(usize::from(query.limit())));
        for row in rows.iter().take(usize::from(query.limit())) {
            items.push(storage::detail(
                &mut tx,
                case,
                MeasureDecisionId::from_uuid(row.get(0)),
                self.hasher.as_ref(),
            )?);
        }
        let next_after_id = if has_more {
            items.last().map(|v| v.command().decision_id)
        } else {
            None
        };
        let at = match items.iter().map(|item| item.recorded_at()).max() {
            Some(floor) => self.now(Some(floor))?,
            None => self.now(None)?,
        };
        append_transaction(
            &mut tx,
            &actor.email,
            "measure_decision.list",
            &format!("case:{case}"),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(MeasureDecisionRecordPage {
            case_id: case,
            items,
            has_more,
            next_after_id,
        })
    }
    fn get(
        &self,
        actor: &Principal,
        case: CaseId,
        id: MeasureDecisionId,
    ) -> Result<MeasureDecisionRecordReceipt, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        authorize(&mut tx, actor, case, false)?;
        let result = storage::detail(&mut tx, case, id, self.hasher.as_ref())?;
        append_transaction(
            &mut tx,
            &actor.email,
            "measure_decision.read",
            &marker(&result),
            self.now(Some(result.recorded_at()))?,
        )?;
        tx.commit().map_err(port)?;
        Ok(result)
    }
    fn get_operation(
        &self,
        actor: &Principal,
        case: CaseId,
        op: MeasureDecisionOperationId,
    ) -> Result<MeasureDecisionRecordReceipt, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        authorize(&mut tx, actor, case, false)?;
        let result = storage::operation(&mut tx, case, op, self.hasher.as_ref())?
            .ok_or(MeasureDecisionError::NotFound)?;
        append_transaction(
            &mut tx,
            &actor.email,
            "measure_decision.operation",
            &marker(&result),
            self.now(Some(result.recorded_at()))?,
        )?;
        tx.commit().map_err(port)?;
        Ok(result)
    }
}

fn marker(receipt: &MeasureDecisionRecordReceipt) -> String {
    match receipt {
        MeasureDecisionRecordReceipt::V1(row) => write::marker(&row.group),
        MeasureDecisionRecordReceipt::V2(row) => record_write::marker(&row.group),
    }
}
