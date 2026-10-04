use super::{receipt::inconsistent, *};
use crate::{
    identity::{IdentityWorkflow, Principal},
    ApplicationError,
};
use domain::{
    cases::CaseId,
    clock::{Clock, OffsetDateTime},
    crypto::DocumentHasher,
    identity::Role,
    procedural_resources::ResourceId,
    resource_hearings::{ResourceHearingId, ResourceHearingRevision},
};
use std::sync::Arc;

pub struct ResourceHearingReadService {
    store: Arc<dyn ResourceHearingReadStore>,
    identity: Arc<dyn IdentityWorkflow>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}

impl ResourceHearingReadService {
    pub fn new(
        store: Arc<dyn ResourceHearingReadStore>,
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

    fn reauthenticate(&self, token: &str, actor: &Principal) -> Result<(), ApplicationError> {
        if self.identity.authenticate(token)? != *actor {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(())
    }

    fn creation(
        &self,
        result: &ResourceHearingCreation,
        case: CaseId,
        resource: ResourceId,
        returned_at: OffsetDateTime,
    ) -> Result<(), ApplicationError> {
        let hearing = &result.hearing;
        if hearing.review.case_id != case
            || hearing.review.command.resource.id != resource
            || hearing.recorded_at > returned_at
        {
            return Err(inconsistent(
                "resource hearing read scope or capture time differs",
            ));
        }
        resource_hearing_creation_matches(self.hasher.as_ref(), result)
    }
}

impl ResourceHearingReadWorkflow for ResourceHearingReadService {
    fn list(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        query: ResourceHearingReadQuery,
    ) -> Result<ResourceHearingPage, ApplicationError> {
        let actor = self.actor(token)?;
        let started_at = self.clock.now();
        valid_time(started_at)?;
        let page = self.store.list(actor.id, case, resource, query)?;
        let returned_at = self.clock.now();
        read_window(started_at, returned_at)?;
        let last = page
            .items
            .last()
            .map(|row| row.hearing.review.command.hearing_id);
        if page.case_id != case
            || page.resource_id != resource
            || page.items.len() > usize::from(query.limit())
            || (page.has_more
                && (page.items.len() != usize::from(query.limit())
                    || page.next_after_id.is_none()
                    || page.next_after_id != last))
            || (!page.has_more && page.next_after_id.is_some())
        {
            return Err(inconsistent(
                "resource hearing page scope, length or continuation differs",
            ));
        }
        let mut prior = query.after_id();
        for row in &page.items {
            let id = row.hearing.review.command.hearing_id;
            if prior.is_some_and(|before| before.as_uuid() >= id.as_uuid()) {
                return Err(inconsistent("resource hearing page order differs"));
            }
            self.creation(row, case, resource, returned_at)?;
            prior = Some(id);
        }
        self.reauthenticate(token, &actor)?;
        Ok(page)
    }

    fn get(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        hearing: ResourceHearingId,
        revision: Option<ResourceHearingRevision>,
    ) -> Result<ResourceHearingCreation, ApplicationError> {
        let actor = self.actor(token)?;
        let started_at = self.clock.now();
        valid_time(started_at)?;
        let result = self
            .store
            .get(actor.id, case, resource, hearing, revision)?;
        let returned_at = self.clock.now();
        read_window(started_at, returned_at)?;
        if result.hearing.review.command.hearing_id != hearing
            || revision.is_some_and(|value| value != result.hearing.revision)
        {
            return Err(inconsistent(
                "resource hearing identity or exact revision differs",
            ));
        }
        self.creation(&result, case, resource, returned_at)?;
        self.reauthenticate(token, &actor)?;
        Ok(result)
    }
}

fn valid_time(at: OffsetDateTime) -> Result<(), ApplicationError> {
    if at.offset() != time::UtcOffset::UTC || !(1..=9999).contains(&at.year()) {
        return Err(inconsistent(
            "resource hearing read clock must be representable UTC",
        ));
    }
    Ok(())
}

fn read_window(
    started_at: OffsetDateTime,
    returned_at: OffsetDateTime,
) -> Result<(), ApplicationError> {
    valid_time(returned_at)?;
    if returned_at < started_at {
        return Err(inconsistent("resource hearing read clock regressed"));
    }
    Ok(())
}
