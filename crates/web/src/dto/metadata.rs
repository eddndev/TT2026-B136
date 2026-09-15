//! Organizational metadata is separate from captured document evidence.

use application::documents::{
    CurrentDocumentMetadata, DocumentMetadata, DocumentMetadataRevision, DocumentOverview,
    MetadataPage, MetadataRevision,
};
use application::ApplicationError;
use domain::{cases::CaseId, crypto::DocumentId};
use serde::{Deserialize, Serialize};
use time::{format_description::well_known::Rfc3339, UtcOffset};

use super::DocumentResponse;
use crate::error::ApiError;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MetadataValuesRequest {
    document_type: Option<String>,
    classification: Option<String>,
    tags: Vec<String>,
}

impl MetadataValuesRequest {
    pub(crate) fn validate(self) -> Result<DocumentMetadata, ApiError> {
        DocumentMetadata::new(
            self.document_type.as_deref(),
            self.classification.as_deref(),
            &self.tags,
        )
        .map_err(|error| ApplicationError::from(error).into())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReplaceMetadataRequest {
    expected_metadata_revision: u32,
    document_type: Option<String>,
    classification: Option<String>,
    tags: Vec<String>,
}

impl ReplaceMetadataRequest {
    pub(crate) fn validate(self) -> Result<(MetadataRevision, DocumentMetadata), ApiError> {
        let metadata = MetadataValuesRequest {
            document_type: self.document_type,
            classification: self.classification,
            tags: self.tags,
        }
        .validate()?;
        Ok((
            MetadataRevision::new(self.expected_metadata_revision),
            metadata,
        ))
    }
}

#[derive(Serialize)]
struct CurrentMetadataResponse {
    metadata_revision: u32,
    document_type: Option<String>,
    classification: Option<String>,
    tags: Vec<String>,
}

impl From<CurrentDocumentMetadata> for CurrentMetadataResponse {
    fn from(current: CurrentDocumentMetadata) -> Self {
        Self {
            metadata_revision: current.metadata_revision.get(),
            document_type: current.values.document_type().map(str::to_owned),
            classification: current.values.classification().map(str::to_owned),
            tags: current.values.tags().to_vec(),
        }
    }
}

#[derive(Serialize)]
pub(crate) struct DocumentOverviewResponse {
    #[serde(flatten)]
    content: DocumentResponse,
    current_metadata: CurrentMetadataResponse,
}

impl From<DocumentOverview> for DocumentOverviewResponse {
    fn from(overview: DocumentOverview) -> Self {
        Self {
            content: overview.content.into(),
            current_metadata: overview.current_metadata.into(),
        }
    }
}

#[derive(Serialize)]
pub(crate) struct MetadataResponse {
    case_id: String,
    id: String,
    #[serde(flatten)]
    current: CurrentMetadataResponse,
}

impl MetadataResponse {
    pub(crate) fn new(case: CaseId, id: DocumentId, current: CurrentDocumentMetadata) -> Self {
        Self {
            case_id: case.to_string(),
            id: id.to_string(),
            current: current.into(),
        }
    }
}

#[derive(Serialize)]
struct MetadataActorResponse {
    id: String,
    email: String,
}

#[derive(Serialize)]
struct MetadataRevisionResponse {
    #[serde(flatten)]
    current: CurrentMetadataResponse,
    metadata_digest: String,
    changed_at: String,
    changed_by: MetadataActorResponse,
}

impl TryFrom<DocumentMetadataRevision> for MetadataRevisionResponse {
    type Error = ApiError;
    fn try_from(revision: DocumentMetadataRevision) -> Result<Self, Self::Error> {
        Ok(Self {
            current: CurrentDocumentMetadata {
                metadata_revision: revision.metadata_revision,
                values: revision.values,
            }
            .into(),
            metadata_digest: revision.metadata_digest.to_hex(),
            changed_at: revision
                .changed_at
                .to_offset(UtcOffset::UTC)
                .format(&Rfc3339)
                .map_err(|_| ApiError::internal())?,
            changed_by: MetadataActorResponse {
                id: revision.changed_by.id.to_string(),
                email: revision.changed_by.email,
            },
        })
    }
}

#[derive(Serialize)]
pub(crate) struct MetadataHistoryResponse {
    case_id: String,
    id: String,
    revisions: Vec<MetadataRevisionResponse>,
    has_more: bool,
    next_before_revision: Option<u32>,
}

impl MetadataHistoryResponse {
    pub(crate) fn new(case: CaseId, id: DocumentId, page: MetadataPage) -> Result<Self, ApiError> {
        Ok(Self {
            case_id: case.to_string(),
            id: id.to_string(),
            revisions: page
                .revisions
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            has_more: page.has_more,
            next_before_revision: page.next_before_revision.map(|revision| revision.get()),
        })
    }
}
