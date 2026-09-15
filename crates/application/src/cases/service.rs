//! Session and role checks precede each actor-scoped transactional command.

use std::sync::Arc;

use domain::cases::{can_create_case, can_manage_members, CaseId, CaseMetadata};
use domain::clock::Clock;
use domain::identity::{Permission, UserId};

use super::{
    CaseAdministrationAction, CaseAdministrationDetail, CaseAdministrationHistoryPage,
    CaseAdministrationHistoryQuery, CaseAdministrationPage, CaseAdministrationQuery,
    CaseAdministrativeStatus, CaseEditableValues, CaseRecord, CaseRepository,
    CaseRevisionExpectation, CaseWorkflow, PenalCaseCreation,
};
use crate::identity::IdentityWorkflow;
use crate::ApplicationError;

/// Combines bearer identity with audited basic and staff case workflows.
pub struct CaseService {
    repository: Arc<dyn CaseRepository>,
    identity: Arc<dyn IdentityWorkflow>,
    clock: Arc<dyn Clock + Send + Sync>,
}
impl CaseService {
    pub fn new(
        repository: Arc<dyn CaseRepository>,
        identity: Arc<dyn IdentityWorkflow>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            repository,
            identity,
            clock,
        }
    }
    fn actor(&self, token: &str, permission: Permission) -> Result<UserId, ApplicationError> {
        let actor = self.identity.authenticate(token)?;
        if !actor.role.allows(permission) {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(actor.id)
    }
    fn require_membership_manager(&self, token: &str) -> Result<UserId, ApplicationError> {
        let actor = self.identity.authenticate(token)?;
        if !can_manage_members(actor.role) {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(actor.id)
    }
}

impl CaseWorkflow for CaseService {
    fn create(
        &self,
        token: &str,
        title: &str,
        reference: &str,
    ) -> Result<CaseRecord, ApplicationError> {
        let actor = self.identity.authenticate(token)?;
        if !can_create_case(actor.role) {
            return Err(ApplicationError::PermissionDenied);
        }
        let metadata = CaseMetadata::new(title, reference)?;
        self.repository
            .create_basic(actor.id, CaseId::new(), metadata, self.clock.now())
    }
    fn list(
        &self,
        token: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<CaseRecord>, ApplicationError> {
        let actor = self.identity.authenticate(token)?;
        super::query::validate_limit(limit)?;
        self.repository
            .list_basic(actor.id, limit, offset, self.clock.now())
    }
    fn get(&self, token: &str, id: CaseId) -> Result<CaseRecord, ApplicationError> {
        let actor = self.identity.authenticate(token)?;
        self.repository.get_basic(actor.id, id, self.clock.now())
    }
    fn assign(&self, token: &str, id: CaseId, user_id: UserId) -> Result<(), ApplicationError> {
        let actor = self.require_membership_manager(token)?;
        self.repository
            .add_member(id, user_id, actor, self.clock.now())
    }
    fn remove(&self, token: &str, id: CaseId, user_id: UserId) -> Result<(), ApplicationError> {
        let actor = self.require_membership_manager(token)?;
        self.repository
            .remove_member(id, user_id, actor, self.clock.now())
    }
    fn register_penal(
        &self,
        token: &str,
        creation: PenalCaseCreation,
    ) -> Result<CaseAdministrationDetail, ApplicationError> {
        let actor = self.actor(token, CaseAdministrationAction::RegisterPenal.permission())?;
        self.repository
            .register_penal(actor, CaseId::new(), creation, self.clock.now())
    }
    fn replace_administration(
        &self,
        token: &str,
        id: CaseId,
        expected: CaseRevisionExpectation,
        values: CaseEditableValues,
    ) -> Result<CaseAdministrationDetail, ApplicationError> {
        let actor = self.actor(token, CaseAdministrationAction::Replace.permission())?;
        self.repository
            .replace_administration(actor, id, expected, values, self.clock.now())
    }
    fn change_administrative_status(
        &self,
        token: &str,
        id: CaseId,
        expected: CaseRevisionExpectation,
        status: CaseAdministrativeStatus,
    ) -> Result<CaseAdministrationDetail, ApplicationError> {
        let actor = self.actor(token, CaseAdministrationAction::ChangeStatus.permission())?;
        self.repository
            .change_administrative_status(actor, id, expected, status, self.clock.now())
    }
    fn list_administrations(
        &self,
        token: &str,
        query: CaseAdministrationQuery,
    ) -> Result<CaseAdministrationPage, ApplicationError> {
        let actor = self.actor(token, CaseAdministrationAction::List.permission())?;
        self.repository
            .list_administrations(actor, query, self.clock.now())
    }
    fn get_administration(
        &self,
        token: &str,
        id: CaseId,
    ) -> Result<CaseAdministrationDetail, ApplicationError> {
        let actor = self.actor(token, CaseAdministrationAction::Read.permission())?;
        self.repository
            .get_administration(actor, id, self.clock.now())
    }
    fn administration_history(
        &self,
        token: &str,
        id: CaseId,
        query: CaseAdministrationHistoryQuery,
    ) -> Result<CaseAdministrationHistoryPage, ApplicationError> {
        let actor = self.actor(token, CaseAdministrationAction::History.permission())?;
        self.repository
            .administration_history(actor, id, query, self.clock.now())
    }
}
