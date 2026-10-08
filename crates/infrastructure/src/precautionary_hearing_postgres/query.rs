use super::{
    audit, authorization::authorize, port, storage, write, PostgresPrecautionaryHearingStore,
};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{identity::Principal, precautionary_hearings::*, ApplicationError};
use domain::{cases::CaseId, precautionary_hearings::*};
impl PrecautionaryHearingReadStore for PostgresPrecautionaryHearingStore {
    fn list(
        &self,
        actor: &Principal,
        case: CaseId,
        query: PrecautionaryHearingReadQuery,
    ) -> Result<PrecautionaryHearingPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        authorize(&mut tx, actor, case, false)?;
        audit::case_intact(&mut tx, case)?;
        let rows=tx.query("SELECT id FROM case_precautionary_hearings WHERE case_id=$1 AND ($2::uuid IS NULL OR id>$2) ORDER BY id LIMIT $3",&[&case.as_uuid(),&query.after_id().map(|v|v.as_uuid()),&(i64::from(query.limit())+1)]).map_err(port)?;
        let has_more = rows.len() > usize::from(query.limit());
        let mut items = Vec::with_capacity(rows.len().min(usize::from(query.limit())));
        for row in rows.iter().take(usize::from(query.limit())) {
            items.push(storage::detail(
                &mut tx,
                case,
                PrecautionaryHearingId::from_uuid(row.get(0)),
                None,
                self.hasher.as_ref(),
            )?);
        }
        let next_after_id = if has_more {
            items.last().map(|v| v.capture.review.command.hearing_id)
        } else {
            None
        };
        let at = match items.iter().map(|item| item.capture.recorded_at).max() {
            Some(floor) => self.now_after(floor)?,
            None => self.now()?,
        };
        append_transaction(
            &mut tx,
            &actor.email,
            "precautionary_hearing.list",
            &format!("case:{case}"),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(PrecautionaryHearingPage {
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
        id: PrecautionaryHearingId,
        revision: Option<PrecautionaryHearingRevision>,
    ) -> Result<PrecautionaryHearingStoredOperation, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        authorize(&mut tx, actor, case, false)?;
        let result = storage::detail(&mut tx, case, id, revision, self.hasher.as_ref())?;
        append_transaction(
            &mut tx,
            &actor.email,
            "precautionary_hearing.read",
            &write::marker(&result.capture),
            self.now_after(result.capture.recorded_at)?,
        )?;
        tx.commit().map_err(port)?;
        Ok(result)
    }
    fn get_operation(
        &self,
        actor: &Principal,
        case: CaseId,
        op: PrecautionaryHearingOperationId,
    ) -> Result<PrecautionaryHearingStoredOperation, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        authorize(&mut tx, actor, case, false)?;
        let result = storage::operation(&mut tx, case, op, self.hasher.as_ref())?
            .ok_or(PrecautionaryHearingError::NotFound)?;
        append_transaction(
            &mut tx,
            &actor.email,
            "precautionary_hearing.operation",
            &write::marker(&result.capture),
            self.now_after(result.capture.recorded_at)?,
        )?;
        tx.commit().map_err(port)?;
        Ok(result)
    }
}
