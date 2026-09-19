use application::document_integrity::{
    DocumentIntegrityFailure, DocumentIntegrityIncident, DocumentIntegrityIncidentId,
    DocumentIntegrityObservation, DocumentIntegrityObservationId,
};
use application::ApplicationError;
use domain::cases::CaseId;
use domain::crypto::{
    DocumentHasher, DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest,
};
use domain::identity::UserId;
use postgres::Row;
use time::{OffsetDateTime, UtcOffset};

use crate::RingSha256Hasher;

pub(super) fn snapshot(observation: &DocumentIntegrityObservation) -> Sha256Digest {
    let record = &observation.record;
    let mut bytes = b"DICS1".to_vec();
    bytes.extend_from_slice(observation.case_id.as_uuid().as_bytes());
    bytes.extend_from_slice(record.id.as_uuid().as_bytes());
    bytes.extend_from_slice(&record.version.get().to_be_bytes());
    bytes.extend_from_slice(&(record.name.as_str().len() as u64).to_be_bytes());
    bytes.extend_from_slice(record.name.as_str().as_bytes());
    bytes.extend_from_slice(record.digest.as_bytes());
    bytes.extend_from_slice(&(record.vault.len() as u64).to_be_bytes());
    bytes.extend_from_slice(RingSha256Hasher.hash_bytes(&record.vault).as_bytes());
    RingSha256Hasher.hash_bytes(&bytes)
}

pub(super) fn capture(incident: &DocumentIntegrityIncident) -> Vec<u8> {
    let mut bytes = b"DINC1".to_vec();
    for id in [
        incident.id.as_uuid(),
        incident.observation_id.as_uuid(),
        incident.case_id.as_uuid(),
        incident.reference.id.as_uuid(),
    ] {
        bytes.extend_from_slice(id.as_bytes());
    }
    bytes.extend_from_slice(&incident.reference.version.get().to_be_bytes());
    bytes.extend_from_slice(incident.requester.as_uuid().as_bytes());
    bytes.push(match incident.failure {
        DocumentIntegrityFailure::MalformedVault => 1,
        DocumentIntegrityFailure::AuthenticationFailed => 2,
        DocumentIntegrityFailure::DigestMismatch => 3,
        DocumentIntegrityFailure::SnapshotChanged => 4,
    });
    for at in [incident.detected_at, incident.recorded_at] {
        bytes.extend_from_slice(&at.unix_timestamp().to_be_bytes());
        bytes.extend_from_slice(&at.nanosecond().to_be_bytes());
    }
    bytes.extend_from_slice(incident.expected_digest.as_bytes());
    bytes.extend_from_slice(incident.observed_snapshot_digest.as_bytes());
    bytes
}

pub(super) fn decode(row: &Row) -> Result<DocumentIntegrityIncident, ApplicationError> {
    let version =
        u32::try_from(row.get::<_, i64>("document_version")).map_err(|_| inconsistent())?;
    let incident = DocumentIntegrityIncident {
        id: DocumentIntegrityIncidentId::from_uuid(row.get("id")),
        observation_id: DocumentIntegrityObservationId::from_uuid(row.get("observation_id")),
        case_id: CaseId::from_uuid(row.get("case_id")),
        reference: DocumentVersionRef {
            id: DocumentId::from_uuid(row.get("document_id")),
            version: DocumentVersion::new(version).map_err(|_| inconsistent())?,
        },
        requester: UserId::from_uuid(row.get("requester")),
        failure: match row.get::<_, String>("failure").as_str() {
            "malformed_vault" => DocumentIntegrityFailure::MalformedVault,
            "authentication_failed" => DocumentIntegrityFailure::AuthenticationFailed,
            "digest_mismatch" => DocumentIntegrityFailure::DigestMismatch,
            "snapshot_changed" => DocumentIntegrityFailure::SnapshotChanged,
            _ => return Err(inconsistent()),
        },
        detected_at: instant(row, "detected_at_seconds", "detected_at_nanoseconds")?,
        recorded_at: instant(row, "recorded_at_seconds", "recorded_at_nanoseconds")?,
        expected_digest: digest(row, "expected_digest")?,
        observed_snapshot_digest: digest(row, "observed_snapshot_digest")?,
    };
    let canonical = capture(&incident);
    if incident.recorded_at < incident.detected_at
        || canonical != row.get::<_, Vec<u8>>("capture_canonical")
        || RingSha256Hasher.hash_bytes(&canonical) != digest(row, "capture_digest")?
    {
        return Err(inconsistent());
    }
    Ok(incident)
}

fn digest(row: &Row, name: &str) -> Result<Sha256Digest, ApplicationError> {
    let bytes: Vec<u8> = row.get(name);
    Sha256Digest::from_bytes(&bytes).map_err(|_| inconsistent())
}

fn instant(row: &Row, seconds: &str, nanos: &str) -> Result<OffsetDateTime, ApplicationError> {
    let at = OffsetDateTime::from_unix_timestamp(row.get(seconds))
        .map_err(|_| inconsistent())?
        .replace_nanosecond(u32::try_from(row.get::<_, i32>(nanos)).map_err(|_| inconsistent())?)
        .map_err(|_| inconsistent())?;
    if !valid_time(at) {
        return Err(inconsistent());
    }
    Ok(at)
}

pub(super) fn valid_time(at: OffsetDateTime) -> bool {
    at.offset() == UtcOffset::UTC && (1..=9999).contains(&at.year())
}

pub(super) fn inconsistent() -> ApplicationError {
    ApplicationError::StoredDocumentInconsistent(
        "stored document integrity incident is inconsistent".into(),
    )
}
