use super::{checks::*, *};
use crate::{
    identity::{IdentityWorkflow, Principal},
    ApplicationError,
};
use domain::{
    clock::{Clock, OffsetDateTime},
    crypto::DocumentHasher,
};
use std::sync::Arc;

pub struct CaseReportService {
    store: Arc<dyn CaseReportStore>,
    identity: Arc<dyn IdentityWorkflow>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}
impl CaseReportService {
    pub fn new(
        store: Arc<dyn CaseReportStore>,
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
        scope(&actor)?;
        Ok(actor)
    }
    fn reauthenticate(&self, token: &str, actor: &Principal) -> Result<(), ApplicationError> {
        if self.identity.authenticate(token)? != *actor {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(())
    }
    fn returned(&self, started: OffsetDateTime) -> Result<OffsetDateTime, ApplicationError> {
        let returned = self.clock.now();
        window(started, returned)?;
        Ok(returned)
    }
}
impl CaseReportWorkflow for CaseReportService {
    fn litigators(
        &self,
        token: &str,
        query: CaseReportLitigatorQuery,
    ) -> Result<CaseReportLitigatorPage, ApplicationError> {
        let actor = self.actor(token)?;
        let expected_scope = scope(&actor)?;
        principal(&actor, expected_scope)?;
        if !(1..=100).contains(&query.limit)
            || query.after_id.is_some_and(|id| id.as_uuid().is_nil())
        {
            return Err(ApplicationError::InvalidInput(
                "report litigator page requires a limit from 1 to 100 and a valid cursor".into(),
            ));
        }
        let started = self.clock.now();
        time(started)?;
        let page = self.store.litigators(&actor, query, started)?;
        let returned = self.returned(started)?;
        time(page.checked_at)?;
        let last = page.litigators.last().map(|value| value.user_id);
        if page.scope != expected_scope
            || page.checked_at < started
            || page.checked_at > returned
            || page.litigators.len() > query.limit as usize
            || (page.has_more
                && (page.litigators.len() != query.limit as usize || page.next_after_id != last))
            || (!page.has_more && page.next_after_id.is_some())
        {
            return Err(inconsistent(
                "report litigator scope, clock, limit or continuation differs",
            ));
        }
        let mut prior = query.after_id.map(|id| id.as_uuid());
        for value in &page.litigators {
            let id = value.user_id.as_uuid();
            if id.is_nil() || prior.is_some_and(|previous| previous >= id) {
                return Err(inconsistent("report litigator identity or order differs"));
            }
            email(&value.email)?;
            prior = Some(id);
        }
        self.reauthenticate(token, &actor)?;
        Ok(page)
    }
    fn request(
        &self,
        token: &str,
        command: CaseReportCommand,
    ) -> Result<CaseReportDetail, ApplicationError> {
        let actor = self.actor(token)?;
        let scope = scope(&actor)?;
        let digest = case_report_request_digest(self.hasher.as_ref(), &actor, scope, &command)?;
        let started = self.clock.now();
        time(started)?;
        let result = self
            .store
            .request(&actor, scope, command.clone(), digest, started)?;
        let returned = self.returned(started)?;
        super::receipt::detail(self.hasher.as_ref(), &result, &actor, result.id, returned)?;
        if result.command != command || result.request_digest != digest {
            return Err(inconsistent(
                "request receipt differs from submitted operation",
            ));
        }
        self.reauthenticate(token, &actor)?;
        Ok(result)
    }
    fn list(
        &self,
        token: &str,
        query: CaseReportQuery,
    ) -> Result<CaseReportPage, ApplicationError> {
        let actor = self.actor(token)?;
        query_check(query)?;
        let started = self.clock.now();
        time(started)?;
        let page = self.store.list(&actor, query, started)?;
        let returned = self.returned(started)?;
        time(page.checked_at)?;
        let last = page.reports.last().map(|value| value.id);
        if page.checked_at < started
            || page.checked_at > returned
            || page.reports.len() > query.limit as usize
            || (page.has_more
                && (page.reports.len() != query.limit as usize || page.next_after_id != last))
            || (!page.has_more && page.next_after_id.is_some())
        {
            return Err(inconsistent(
                "report page clock, limit or continuation differs",
            ));
        }
        let mut prior = query.after_id;
        for value in &page.reports {
            super::receipt::detail(
                self.hasher.as_ref(),
                value,
                &actor,
                value.id,
                page.checked_at,
            )?;
            if prior.is_some_and(|id| id >= value.id)
                || (query.unread_only && value.notice.as_ref().is_none_or(|v| v.read_at.is_some()))
            {
                return Err(inconsistent(
                    "report page order, cursor or unread filter differs",
                ));
            }
            prior = Some(value.id);
        }
        self.reauthenticate(token, &actor)?;
        Ok(page)
    }
    fn get(&self, token: &str, id: CaseReportId) -> Result<CaseReportDetail, ApplicationError> {
        let actor = self.actor(token)?;
        let started = self.clock.now();
        time(started)?;
        let result = self.store.get(&actor, id, started)?;
        let returned = self.returned(started)?;
        super::receipt::detail(self.hasher.as_ref(), &result, &actor, id, returned)?;
        self.reauthenticate(token, &actor)?;
        Ok(result)
    }
    fn download(
        &self,
        token: &str,
        id: CaseReportId,
        format: CaseReportFormat,
    ) -> Result<CaseReportDownload, ApplicationError> {
        let actor = self.actor(token)?;
        let started = self.clock.now();
        time(started)?;
        let result = self.store.download(&actor, id, format, started)?;
        let returned = self.returned(started)?;
        super::receipt::download(self.hasher.as_ref(), &result, &actor, id, format, returned)?;
        self.reauthenticate(token, &actor)?;
        Ok(result)
    }
    fn acknowledge_notice(
        &self,
        token: &str,
        id: CaseReportId,
    ) -> Result<CaseReportDetail, ApplicationError> {
        let actor = self.actor(token)?;
        let started = self.clock.now();
        time(started)?;
        let result = self.store.acknowledge_notice(&actor, id, started)?;
        let returned = self.returned(started)?;
        super::receipt::detail(self.hasher.as_ref(), &result, &actor, id, returned)?;
        if result
            .notice
            .as_ref()
            .is_none_or(|notice| notice.read_at.is_none())
        {
            return Err(inconsistent(
                "report notice acknowledgement was not persisted",
            ));
        }
        self.reauthenticate(token, &actor)?;
        Ok(result)
    }
}
fn query_check(value: CaseReportQuery) -> Result<(), ApplicationError> {
    super::checks::query(value)
}
