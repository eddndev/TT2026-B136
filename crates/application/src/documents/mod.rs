//! Offline cryptographic workflows and authenticated case-scoped documents.

mod case_content;
mod case_port;
mod case_service;
pub mod format;
mod individual_evidence;
mod metadata;
mod metadata_filter;
mod model;
mod port;
mod processor;
mod query;
mod service;
mod signing_manifest;
mod upload_admission;
mod validation;
mod version;

pub use case_port::{CaseDocumentStore, CaseDocumentSummary, CaseDocumentWorkflow, DocumentAction};
pub use case_service::CaseDocumentService;
pub use domain::crypto::DocumentVersionRef;
pub use individual_evidence::IndividualSigningEvidence;
pub use model::{DocumentRecord, DocumentSummary, EvidenceExport, SealedEvidence};
pub use port::{DocumentRepository, DocumentWorkflow};
pub use processor::{
    DocumentProcessor, DocumentProcessorPorts, EvidenceMaterial, ExternalSignerMaterial,
    SealedExternal,
};
pub use query::{DocumentPage, DocumentQuery};
pub use service::{DocumentWorkflowPorts, LocalDocumentWorkflow};
pub use signing_manifest::{
    reserve_document_name, ManifestExport, SigningManifest, MANIFEST_LIMIT, TSA_CRL_PEM_LIMIT,
};
pub use validation::{validate_record_with_ports, DocumentValidationPorts};
pub use version::{VersionPage, VersionQuery, VersionSelection};

pub use metadata::{
    metadata_digest, CurrentDocumentMetadata, DocumentMetadata, DocumentMetadataRevision,
    DocumentOverview, MetadataActorSnapshot, MetadataPage, MetadataQuery, MetadataRevision,
};
pub use metadata_filter::DocumentMetadataFilter;

pub use format::{
    DocumentFormatBatch, DocumentFormatBatchValidator, DocumentFormatInput, StageDocumentFormat,
    StageFormatPolicy, StageSupportReadLimits,
};
mod content_validation;

pub use upload_admission::{
    AdmittedDocumentFormat, DocumentUploadAdmission, DocumentUploadError, MAX_DOCUMENT_UPLOAD_BYTES,
};

mod exact_support;
pub(crate) use exact_support::admit_exact_stage_support;
