use super::{read_inventory::ReadInventory, record_read_error::inconsistent, *};
use crate::{
    identity::{IdentityWorkflow, Principal},
    ApplicationError,
};
use domain::{
    cases::CaseId,
    clock::{Clock, OffsetDateTime},
    crypto::DocumentHasher,
    identity::Role,
    precautionary_hearings::{MeasureId, PrecautionaryMeasureRef},
};
use std::sync::Arc;

pub struct MeasureRecordReadService {
    store: Arc<dyn MeasureRecordReadStore>,
    identity: Arc<dyn IdentityWorkflow>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}

impl MeasureRecordReadService {
    pub fn new(
        store: Arc<dyn MeasureRecordReadStore>,
        identity: Arc<dyn IdentityWorkflow>,
        hasher: Arc<dyn DocumentHasher + Send + Sync>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            store,
            identity,
            hasher,
            clock,
        }
    }

    fn actor(&self, token: &str) -> Result<Principal, ApplicationError> {
        let actor = self.identity.authenticate(token)?;
        if actor.role == Role::Client {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(actor)
    }

    fn observation(
        &self,
        lower: Option<OffsetDateTime>,
    ) -> Result<OffsetDateTime, ApplicationError> {
        let at = self.clock.now();
        if at.offset() != time::UtcOffset::UTC || !(1..=9999).contains(&at.year()) {
            return Err(inconsistent("record read clock must use supported UTC"));
        }
        if lower.is_some_and(|lower| at < lower) {
            return Err(inconsistent("record read clock regressed"));
        }
        Ok(at)
    }

    fn detail(
        &self,
        row: &MeasureRecordDetail,
        case_id: CaseId,
    ) -> Result<OffsetDateTime, ApplicationError> {
        if row.case_id != case_id {
            return Err(inconsistent("record read case differs"));
        }
        let checked = resolve_measure_records_with_decision_history(
            self.hasher.as_ref(),
            case_id,
            &[row.reference],
            &row.record_history,
        )
        .map_err(|error| inconsistent(error.to_string()))?;
        let target = checked
            .targets()
            .first()
            .ok_or_else(|| inconsistent("record read target is absent"))?;
        if target.reference() != row.reference || target.record() != &row.record {
            return Err(inconsistent(
                "record differs from its complete owning evidence",
            ));
        }
        Ok(target.recorded_at())
    }

    fn disclose(
        &self,
        token: &str,
        actor: &Principal,
        floor: OffsetDateTime,
    ) -> Result<(), ApplicationError> {
        if self.identity.authenticate(token)? != *actor {
            return Err(ApplicationError::InvalidSession);
        }
        self.observation(Some(floor))?;
        Ok(())
    }
}

impl MeasureRecordReadWorkflow for MeasureRecordReadService {
    fn list(
        &self,
        token: &str,
        case_id: CaseId,
        query: MeasureRecordReadQuery,
    ) -> Result<MeasureRecordPage, ApplicationError> {
        let actor = self.actor(token)?;
        let mut floor = self.observation(None)?;
        let page = self.store.list(&actor, case_id, query)?;
        let last = page.items.last().map(|row| row.reference.id());
        if page.case_id != case_id
            || page.items.len() > usize::from(query.limit())
            || (page.has_more
                && (page.items.len() != usize::from(query.limit())
                    || page.next_after_id.is_none()
                    || page.next_after_id != last))
            || (!page.has_more && page.next_after_id.is_some())
        {
            return Err(inconsistent(
                "record page scope, length or continuation differs",
            ));
        }
        let mut prior = query.after_id();
        let mut inventory = ReadInventory::default();
        for row in &page.items {
            let id = row.reference.id();
            if prior.is_some_and(|before| before.as_uuid() >= id.as_uuid()) {
                return Err(inconsistent("record page order differs"));
            }
            floor = floor.max(self.detail(row, case_id)?);
            inventory
                .add_history(&row.record_history)
                .map_err(|error| inconsistent(error.to_string()))?;
            prior = Some(id);
        }
        self.disclose(token, &actor, floor)?;
        Ok(page)
    }

    fn get(
        &self,
        token: &str,
        case_id: CaseId,
        id: MeasureId,
    ) -> Result<MeasureRecordDetail, ApplicationError> {
        let actor = self.actor(token)?;
        let started = self.observation(None)?;
        let result = self.store.get(&actor, case_id, id)?;
        if result.reference.id() != id {
            return Err(inconsistent("current record identity differs"));
        }
        let captured_at = self.detail(&result, case_id)?;
        self.disclose(token, &actor, started.max(captured_at))?;
        Ok(result)
    }

    fn exact(
        &self,
        token: &str,
        case_id: CaseId,
        reference: PrecautionaryMeasureRef,
    ) -> Result<MeasureRecordDetail, ApplicationError> {
        let actor = self.actor(token)?;
        let started = self.observation(None)?;
        let result = self.store.exact(&actor, case_id, reference)?;
        if result.reference != reference {
            return Err(inconsistent("exact record selection differs"));
        }
        let captured_at = self.detail(&result, case_id)?;
        self.disclose(token, &actor, started.max(captured_at))?;
        Ok(result)
    }
}
