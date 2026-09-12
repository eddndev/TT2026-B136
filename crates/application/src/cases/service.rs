//! Case authorization belongs to the use cases, independent of HTTP.

use std::sync::Arc;

use domain::cases::{can_create_case, can_manage_members, can_read_case, CaseId, CaseMetadata};
use domain::identity::UserId;

use super::{CaseAccess, CaseRecord, CaseRepository, CaseWorkflow};
use crate::identity::{IdentityWorkflow, Principal};
use crate::ApplicationError;

/// Combines current authenticated identity with durable case visibility.
pub struct CaseService {
    repository: Arc<dyn CaseRepository>,
    identity: Arc<dyn IdentityWorkflow>,
}

impl CaseService {
    pub fn new(repository: Arc<dyn CaseRepository>, identity: Arc<dyn IdentityWorkflow>) -> Self {
        Self {
            repository,
            identity,
        }
    }

    fn access(actor: &Principal) -> CaseAccess {
        if can_read_case(actor.role, false) {
            CaseAccess::All
        } else {
            CaseAccess::Assigned(actor.id)
        }
    }

    fn require_membership_manager(&self, token: &str) -> Result<(), ApplicationError> {
        let actor = self.identity.authenticate(token)?;
        if !can_manage_members(actor.role) {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(())
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
        let record = CaseRecord {
            id: CaseId::new(),
            title: metadata.title().to_owned(),
            reference: metadata.reference().to_owned(),
            created_by: actor.id,
        };
        self.repository.insert(record.clone())?;
        Ok(record)
    }

    fn list(
        &self,
        token: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<CaseRecord>, ApplicationError> {
        let actor = self.identity.authenticate(token)?;
        if !(1..=100).contains(&limit) {
            return Err(ApplicationError::InvalidInput(
                "case limit must be between 1 and 100".into(),
            ));
        }
        self.repository.list(Self::access(&actor), limit, offset)
    }

    fn get(&self, token: &str, id: CaseId) -> Result<CaseRecord, ApplicationError> {
        let actor = self.identity.authenticate(token)?;
        self.repository
            .find(id, Self::access(&actor))?
            .ok_or(ApplicationError::CaseNotFound)
    }

    fn assign(&self, token: &str, id: CaseId, user_id: UserId) -> Result<(), ApplicationError> {
        self.require_membership_manager(token)?;
        self.repository.add_member(id, user_id)
    }

    fn remove(&self, token: &str, id: CaseId, user_id: UserId) -> Result<(), ApplicationError> {
        self.require_membership_manager(token)?;
        self.repository.remove_member(id, user_id)
    }
}
