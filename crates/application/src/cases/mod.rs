//! Authenticated case metadata and membership use cases.
//!
//! Policy and storage boundaries are recorded in
//! docs/adr/0014-case-membership-and-isolation.md.

mod port;
mod service;

use domain::cases::CaseId;
use domain::identity::UserId;
use serde::{Deserialize, Serialize};

pub use port::{CaseRepository, CaseWorkflow};
pub use service::CaseService;

/// Persisted case metadata, independent of document encryption records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseRecord {
    pub id: CaseId,
    pub title: String,
    pub reference: String,
    pub created_by: UserId,
}

/// Visibility selected by the application from an authenticated identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseAccess {
    /// Owner access to all case metadata.
    All,
    /// Only cases with a current assignment for this user.
    Assigned(UserId),
}
