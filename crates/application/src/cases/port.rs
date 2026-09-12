//! Inbound case workflow and durable membership boundary.

use domain::cases::CaseId;
use domain::identity::UserId;

use super::{CaseAccess, CaseRecord};
use crate::ApplicationError;

/// Authenticates each call and enforces current role and case visibility.
pub trait CaseWorkflow: Send + Sync {
    fn create(
        &self,
        access_token: &str,
        title: &str,
        reference: &str,
    ) -> Result<CaseRecord, ApplicationError>;
    fn list(
        &self,
        access_token: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<CaseRecord>, ApplicationError>;
    fn get(&self, access_token: &str, id: CaseId) -> Result<CaseRecord, ApplicationError>;
    fn assign(
        &self,
        access_token: &str,
        id: CaseId,
        user_id: UserId,
    ) -> Result<(), ApplicationError>;
    fn remove(
        &self,
        access_token: &str,
        id: CaseId,
        user_id: UserId,
    ) -> Result<(), ApplicationError>;
}

/// Stores case metadata and checks membership within each read query.
pub trait CaseRepository: Send + Sync {
    /// Atomically creates the case and its creator's initial assignment.
    fn insert(&self, record: CaseRecord) -> Result<(), ApplicationError>;
    /// Filters before applying stable pagination; never returns hidden cases.
    fn list(
        &self,
        access: CaseAccess,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<CaseRecord>, ApplicationError>;
    /// Returns None for both missing cases and cases outside the given scope.
    fn find(&self, id: CaseId, access: CaseAccess) -> Result<Option<CaseRecord>, ApplicationError>;
    /// Idempotently assigns an existing active user to an existing case.
    fn add_member(&self, id: CaseId, user_id: UserId) -> Result<(), ApplicationError>;
    /// Idempotently removes an assignment; fails if the case does not exist.
    fn remove_member(&self, id: CaseId, user_id: UserId) -> Result<(), ApplicationError>;
}
