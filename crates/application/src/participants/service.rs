use std::sync::Arc;

use domain::cases::CaseId;
use domain::clock::Clock;
use domain::identity::UserId;

use super::{
    DirectoryStatus, ParticipantAction, ParticipantDetail, ParticipantHistoryPage,
    ParticipantHistoryQuery, ParticipantId, ParticipantPage, ParticipantQuery, ParticipantRevision,
    ParticipantSnapshot, ParticipantStore, ParticipantValues, ParticipantWorkflow,
};
use crate::identity::IdentityWorkflow;
use crate::ApplicationError;

/// Authenticates directory requests and delegates audited persistence as one call.
pub struct ParticipantService {
    store: Arc<dyn ParticipantStore>,
    identity: Arc<dyn IdentityWorkflow>,
    clock: Arc<dyn Clock + Send + Sync>,
}

impl ParticipantService {
    pub fn new(
        store: Arc<dyn ParticipantStore>,
        identity: Arc<dyn IdentityWorkflow>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            store,
            identity,
            clock,
        }
    }

    fn actor(&self, token: &str, action: ParticipantAction) -> Result<UserId, ApplicationError> {
        let actor = self.identity.authenticate(token)?;
        if !actor.role.allows(action.permission()) {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(actor.id)
    }
}

impl ParticipantWorkflow for ParticipantService {
    fn create(
        &self,
        token: &str,
        case_id: CaseId,
        display_name: &str,
        procedural_role: &str,
        organization: Option<&str>,
        legal_status: Option<&str>,
    ) -> Result<ParticipantSnapshot, ApplicationError> {
        let actor = self.actor(token, ParticipantAction::Create)?;
        let values = ParticipantValues::new(
            display_name,
            procedural_role,
            organization,
            legal_status,
            DirectoryStatus::Active,
        )?;
        self.store.create(
            actor,
            case_id,
            ParticipantId::new(),
            values,
            self.clock.now(),
        )
    }

    fn replace(
        &self,
        token: &str,
        case_id: CaseId,
        id: ParticipantId,
        expected_revision: ParticipantRevision,
        values: ParticipantValues,
    ) -> Result<ParticipantSnapshot, ApplicationError> {
        let actor = self.actor(token, ParticipantAction::Replace)?;
        self.store.replace(
            actor,
            case_id,
            id,
            expected_revision,
            values,
            self.clock.now(),
        )
    }

    fn change_status(
        &self,
        token: &str,
        case_id: CaseId,
        id: ParticipantId,
        expected_revision: ParticipantRevision,
        status: DirectoryStatus,
    ) -> Result<ParticipantDetail, ApplicationError> {
        let actor = self.actor(token, ParticipantAction::ChangeStatus)?;
        self.store.change_status(
            actor,
            case_id,
            id,
            expected_revision,
            status,
            self.clock.now(),
        )
    }

    fn list(
        &self,
        token: &str,
        case_id: CaseId,
        query: ParticipantQuery,
    ) -> Result<ParticipantPage, ApplicationError> {
        let actor = self.actor(token, ParticipantAction::List)?;
        self.store.list(actor, case_id, query, self.clock.now())
    }

    fn get(
        &self,
        token: &str,
        case_id: CaseId,
        id: ParticipantId,
    ) -> Result<ParticipantDetail, ApplicationError> {
        let actor = self.actor(token, ParticipantAction::Read)?;
        self.store.get(actor, case_id, id, self.clock.now())
    }

    fn get_revision(
        &self,
        token: &str,
        case_id: CaseId,
        id: ParticipantId,
        revision: ParticipantRevision,
    ) -> Result<ParticipantDetail, ApplicationError> {
        let actor = self.actor(token, ParticipantAction::Read)?;
        self.store
            .get_revision(actor, case_id, id, revision, self.clock.now())
    }
    fn history(
        &self,
        token: &str,
        case_id: CaseId,
        id: ParticipantId,
        query: ParticipantHistoryQuery,
    ) -> Result<ParticipantHistoryPage, ApplicationError> {
        let actor = self.actor(token, ParticipantAction::History)?;
        self.store
            .history(actor, case_id, id, query, self.clock.now())
    }
}
