use application::{procedural_resources::ResourceId, resource_activities::*, ApplicationError};
use domain::{cases::CaseId, clock::OffsetDateTime, identity::UserId};
use std::sync::Arc;

pub struct BeforeCommit {
    pub store: Arc<dyn ResourceActivityStore>,
    pub callback: Box<dyn Fn() + Send + Sync>,
}
impl ResourceActivityStore for BeforeCommit {
    fn list(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        query: ResourceActivityQuery,
        at: OffsetDateTime,
    ) -> Result<ResourceActivityPage, ApplicationError> {
        self.store.list(actor, case, resource, query, at)
    }
    fn get(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        id: ResourceActivityId,
        revision: Option<ResourceActivityRevision>,
        at: OffsetDateTime,
    ) -> Result<ResourceActivityView, ApplicationError> {
        self.store.get(actor, case, resource, id, revision, at)
    }
    fn history(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        id: ResourceActivityId,
        query: ResourceActivityHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<ResourceActivityHistoryPage, ApplicationError> {
        self.store.history(actor, case, resource, id, query, at)
    }
    fn prepare(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        command: &ResourceActivityCommand,
    ) -> Result<ResourceActivityPreparation, ApplicationError> {
        self.store.prepare(actor, case, resource, command)
    }
    fn commit(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        prepared: PreparedResourceActivityChange,
    ) -> Result<ResourceActivityDetail, ApplicationError> {
        (self.callback)();
        self.store.commit(actor, case, resource, prepared)
    }
}
