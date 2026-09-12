use application::documents::DocumentRecord;
use application::ApplicationError;
use domain::cases::CaseId;
use domain::crypto::{DocumentId, DocumentVersion, Sha256Digest};
use postgres::error::SqlState;
use postgres::{Row, Transaction};

pub(super) fn insert(
    transaction: &mut Transaction<'_>,
    case: CaseId,
    record: &DocumentRecord,
) -> Result<(), ApplicationError> {
    transaction
        .execute(
            "INSERT INTO documents(id,case_id,version,name,digest,vault) VALUES($1,$2,$3,$4,$5,$6)",
            &[
                &record.id.as_uuid(),
                &case.as_uuid(),
                &i64::from(record.version.get()),
                &record.name,
                &&record.digest.as_bytes()[..],
                &record.vault,
            ],
        )
        .map(|_| ())
        .map_err(|error| {
            if error.code() == Some(&SqlState::UNIQUE_VIOLATION) {
                ApplicationError::DocumentAlreadyExists(record.id.to_string())
            } else {
                port_error(error)
            }
        })
}

pub(super) fn load(
    transaction: &mut Transaction<'_>,
    case: CaseId,
    id: DocumentId,
) -> Result<DocumentRecord, ApplicationError> {
    let row = transaction.query_opt("SELECT id,version,name,digest,vault,evidence FROM documents WHERE id=$1 AND case_id=$2",
        &[&id.as_uuid(), &case.as_uuid()]).map_err(port_error)?.ok_or_else(|| ApplicationError::DocumentNotFound(id.to_string()))?;
    decode_record(row)
}

pub(crate) fn decode_record(row: Row) -> Result<DocumentRecord, ApplicationError> {
    let version: i64 = row.get("version");
    let version = u32::try_from(version)
        .map_err(|_| ApplicationError::Port("invalid stored document version".into()))?;
    let digest: Vec<u8> = row.get("digest");
    let digest: [u8; 32] = digest
        .try_into()
        .map_err(|_| ApplicationError::Port("invalid stored document digest".into()))?;
    let mut record = DocumentRecord::pending(
        DocumentId::from_uuid(row.get("id")),
        DocumentVersion::new(version)?,
        row.get("name"),
        Sha256Digest::from_array(digest),
        row.get("vault"),
    )?;
    let json: Option<serde_json::Value> = row.get("evidence");
    if let Some(json) = json {
        record.seal(crate::documents::decode_evidence(json)?)?;
    }
    Ok(record)
}

pub(super) fn unchanged_document(
    current: &DocumentRecord,
    candidate: &DocumentRecord,
) -> Result<(), ApplicationError> {
    if current.id != candidate.id
        || current.version != candidate.version
        || current.name != candidate.name
        || current.digest != candidate.digest
        || current.vault != candidate.vault
    {
        return Err(ApplicationError::StoredDocumentInconsistent(
            "prepared document changed immutable fields".into(),
        ));
    }
    Ok(())
}

pub(super) fn port_error(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("document database: {error}"))
}
