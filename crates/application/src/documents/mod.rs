//! Stored-document model, outbound repository port, and application workflow.

mod model;
mod port;
mod service;

pub use model::{DocumentRecord, DocumentSummary, EvidenceExport, SealedEvidence};
pub use port::{DocumentRepository, DocumentWorkflow};
pub use service::{DocumentWorkflowPorts, EvidenceMaterial, LocalDocumentWorkflow};
