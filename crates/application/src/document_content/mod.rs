//! Exact document bytes released only after integrity and audited authorization.

mod model;
mod service;

pub use model::DocumentContent;
pub use service::DocumentContentService;

use crate::ApplicationError;
use domain::{cases::CaseId, crypto::DocumentVersionRef};

pub const MAX_DOCUMENT_CONTENT_BYTES: usize = 16 * 1024 * 1024;

pub trait DocumentContentWorkflow: Send + Sync {
    fn content_version(
        &self,
        token: &str,
        case_id: CaseId,
        reference: DocumentVersionRef,
    ) -> Result<DocumentContent, ApplicationError>;
}
