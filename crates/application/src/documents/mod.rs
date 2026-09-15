//! Offline cryptographic workflows and authenticated case-scoped documents.

mod case_port;
mod case_service;
mod model;
mod port;
mod processor;
mod query;
mod service;
mod validation;

pub use case_port::{CaseDocumentStore, CaseDocumentSummary, CaseDocumentWorkflow, DocumentAction};
pub use case_service::CaseDocumentService;
pub use model::{DocumentRecord, DocumentSummary, EvidenceExport, SealedEvidence};
pub use port::{DocumentRepository, DocumentWorkflow};
pub use processor::{DocumentProcessor, DocumentProcessorPorts, EvidenceMaterial};
pub use query::{DocumentPage, DocumentQuery};
pub use service::{DocumentWorkflowPorts, LocalDocumentWorkflow};
pub use validation::{validate_record_with_ports, DocumentValidationPorts};
