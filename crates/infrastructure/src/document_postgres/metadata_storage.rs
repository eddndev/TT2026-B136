use application::documents::{
    metadata_digest, CurrentDocumentMetadata, DocumentMetadata, DocumentMetadataRevision,
    MetadataActorSnapshot, MetadataRevision,
};
use application::identity::Principal;
use application::ApplicationError;
use domain::cases::CaseId;
use domain::crypto::{DocumentId, Sha256Digest};
use domain::identity::UserId;
use postgres::{Row, Transaction};
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

use super::storage::port_error;
use crate::RingSha256Hasher;

pub(super) fn require_document(
    transaction: &mut Transaction<'_>,
    case: CaseId,
    id: DocumentId,
) -> Result<(), ApplicationError> {
    transaction
        .query_opt(
            "SELECT id FROM document_series WHERE id=$1 AND case_id=$2",
            &[&id.as_uuid(), &case.as_uuid()],
        )
        .map_err(port_error)?
        .ok_or_else(|| ApplicationError::DocumentNotFound(id.to_string()))?;
    Ok(())
}

pub(super) fn current(
    transaction: &mut Transaction<'_>,
    id: DocumentId,
) -> Result<CurrentDocumentMetadata, ApplicationError> {
    let row = transaction.query_opt(
        "SELECT metadata_revision,document_type,classification,tags,metadata_digest,changed_at,changed_by,changed_by_email
         FROM document_metadata_revisions WHERE document_id=$1 ORDER BY metadata_revision DESC LIMIT 1",
        &[&id.as_uuid()],
    ).map_err(port_error)?;
    row.as_ref().map(current_row).unwrap_or_else(|| Ok(empty()))
}

pub(super) fn current_row(row: &Row) -> Result<CurrentDocumentMetadata, ApplicationError> {
    if row
        .try_get::<_, Option<i64>>("metadata_revision")
        .map_err(inconsistent)?
        .is_none()
    {
        return Ok(empty());
    }
    let revision = decode(row)?;
    Ok(CurrentDocumentMetadata {
        metadata_revision: revision.metadata_revision,
        values: revision.values,
    })
}

fn empty() -> CurrentDocumentMetadata {
    CurrentDocumentMetadata {
        metadata_revision: MetadataRevision::unclassified(),
        values: DocumentMetadata::empty(),
    }
}

pub(super) fn decode(row: &Row) -> Result<DocumentMetadataRevision, ApplicationError> {
    let raw_revision: i64 = row.try_get("metadata_revision").map_err(inconsistent)?;
    let revision = u32::try_from(raw_revision).map_err(inconsistent)?;
    if revision == 0 {
        return Err(inconsistent("stored metadata revision must be positive"));
    }
    let kind: Option<String> = row.try_get("document_type").map_err(inconsistent)?;
    let class: Option<String> = row.try_get("classification").map_err(inconsistent)?;
    let tags: Vec<String> = row.try_get("tags").map_err(inconsistent)?;
    let values =
        DocumentMetadata::new(kind.as_deref(), class.as_deref(), &tags).map_err(inconsistent)?;
    if values.document_type() != kind.as_deref()
        || values.classification() != class.as_deref()
        || values.tags() != tags
    {
        return Err(inconsistent("stored metadata values are not canonical"));
    }
    let digest: Vec<u8> = row.try_get("metadata_digest").map_err(inconsistent)?;
    let digest: [u8; 32] = digest
        .try_into()
        .map_err(|_| inconsistent("invalid metadata digest length"))?;
    let digest = Sha256Digest::from_array(digest);
    if metadata_digest(&RingSha256Hasher::new(), &values) != digest {
        return Err(inconsistent("stored metadata digest disagrees with values"));
    }
    let timestamp: String = row.try_get("changed_at").map_err(inconsistent)?;
    let changed_at = OffsetDateTime::parse(&timestamp, &Rfc3339).map_err(inconsistent)?;
    if timestamp != format_time(changed_at)? {
        return Err(inconsistent(
            "stored metadata timestamp is not canonical UTC",
        ));
    }
    Ok(DocumentMetadataRevision {
        metadata_revision: MetadataRevision::new(revision),
        values,
        metadata_digest: digest,
        changed_at,
        changed_by: MetadataActorSnapshot {
            id: UserId::from_uuid(row.try_get("changed_by").map_err(inconsistent)?),
            email: row.try_get("changed_by_email").map_err(inconsistent)?,
        },
    })
}

pub(super) fn insert(
    transaction: &mut Transaction<'_>,
    id: DocumentId,
    revision: MetadataRevision,
    values: DocumentMetadata,
    principal: &Principal,
    at: OffsetDateTime,
) -> Result<CurrentDocumentMetadata, ApplicationError> {
    let digest = metadata_digest(&RingSha256Hasher::new(), &values);
    let timestamp = format_time(at)?;
    transaction.execute(
        "INSERT INTO document_metadata_revisions(document_id,metadata_revision,document_type,classification,tags,metadata_digest,changed_at,changed_by,changed_by_email)
         VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        &[&id.as_uuid(),&i64::from(revision.get()),&values.document_type(),&values.classification(),
            &values.tags(),&&digest.as_bytes()[..],&timestamp,&principal.id.as_uuid(),&principal.email],
    ).map_err(port_error)?;
    Ok(CurrentDocumentMetadata {
        metadata_revision: revision,
        values,
    })
}

pub(super) fn resource(case: CaseId, id: DocumentId, current: &CurrentDocumentMetadata) -> String {
    format!(
        "case:{case}:document:{id}:metadata:{}:sha256:{}",
        current.metadata_revision.get(),
        metadata_digest(&RingSha256Hasher::new(), &current.values).to_hex()
    )
}

fn format_time(at: OffsetDateTime) -> Result<String, ApplicationError> {
    at.to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .map_err(inconsistent)
}

fn inconsistent(error: impl std::fmt::Display) -> ApplicationError {
    ApplicationError::StoredDocumentMetadataInconsistent(error.to_string())
}
