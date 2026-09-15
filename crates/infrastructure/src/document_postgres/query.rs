//! Metadata projections avoid loading ciphertext or captured evidence for reads.

use application::documents::{CaseDocumentSummary, DocumentPage, DocumentQuery, DocumentSummary};
use application::ApplicationError;
use domain::cases::CaseId;
use domain::crypto::{DocumentId, DocumentVersion, Sha256Digest};
use postgres::{Row, Transaction};

use super::storage::port_error;

pub(super) fn list(
    transaction: &mut Transaction<'_>,
    case: CaseId,
    query: DocumentQuery,
) -> Result<DocumentPage, ApplicationError> {
    let rows = transaction
        .query(
            "SELECT id,version,name,digest,(evidence IS NOT NULL) AS sealed
         FROM documents WHERE case_id=$1
         AND ($2::text IS NULL OR strpos(lower(name), lower($2::text)) > 0)
         AND ($3::boolean IS NULL OR (evidence IS NOT NULL)=$3)
         ORDER BY id LIMIT $4 OFFSET $5",
            &[
                &case.as_uuid(),
                &query.name(),
                &query.sealed(),
                &(i64::from(query.limit()) + 1),
                &i64::from(query.offset()),
            ],
        )
        .map_err(port_error)?;
    let mut documents = rows
        .into_iter()
        .map(|row| summary(case, row))
        .collect::<Result<Vec<_>, _>>()?;
    let has_more = documents.len() > query.limit() as usize;
    documents.truncate(query.limit() as usize);
    Ok(DocumentPage {
        documents,
        has_more,
    })
}

pub(super) fn get(
    transaction: &mut Transaction<'_>,
    case: CaseId,
    id: DocumentId,
) -> Result<CaseDocumentSummary, ApplicationError> {
    let row = transaction
        .query_opt(
            "SELECT id,version,name,digest,(evidence IS NOT NULL) AS sealed
         FROM documents WHERE id=$1 AND case_id=$2",
            &[&id.as_uuid(), &case.as_uuid()],
        )
        .map_err(port_error)?
        .ok_or_else(|| ApplicationError::DocumentNotFound(id.to_string()))?;
    summary(case, row)
}

fn summary(case: CaseId, row: Row) -> Result<CaseDocumentSummary, ApplicationError> {
    let version: i64 = row.get("version");
    let version = u32::try_from(version)
        .map_err(|_| ApplicationError::Port("invalid stored document version".into()))?;
    let digest: Vec<u8> = row.get("digest");
    let digest: [u8; 32] = digest
        .try_into()
        .map_err(|_| ApplicationError::Port("invalid stored document digest".into()))?;
    Ok(CaseDocumentSummary {
        case_id: case,
        document: DocumentSummary {
            id: DocumentId::from_uuid(row.get("id")),
            version: DocumentVersion::new(version)?,
            name: row.get("name"),
            digest_hex: Sha256Digest::from_array(digest).to_hex(),
            sealed: row.get("sealed"),
        },
    })
}
