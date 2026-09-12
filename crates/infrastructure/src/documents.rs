//! Atomic JSON-file storage for encrypted document records.

use std::fs::{File, OpenOptions};
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};

use application::documents::{DocumentRecord, DocumentRepository, SealedEvidence};
use application::ApplicationError;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use domain::crypto::{DocumentId, DocumentVersion, Sha256Digest};
use fs2::FileExt;
use serde::{Deserialize, Serialize};

/// Repository storing one encrypted document record per JSON file.
#[derive(Clone, Debug)]
pub struct FileDocumentRepository {
    root: PathBuf,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredRecord {
    id: DocumentId,
    version: DocumentVersion,
    name: String,
    digest: String,
    vault_base64: String,
    evidence: Option<StoredEvidence>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredEvidence {
    signature_base64: String,
    timestamp_token_base64: String,
    signer_certificate_base64: String,
    issuer_certificate_base64: String,
    crl_base64: String,
    tsa_chain_base64: Option<String>,
    openssl_version: String,
}

impl FileDocumentRepository {
    /// Creates the storage directory when it does not exist.
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, ApplicationError> {
        let root = root.into();
        std::fs::create_dir_all(&root).map_err(|error| storage_error(&root, error))?;
        Ok(Self { root })
    }

    fn record_path(&self, id: DocumentId) -> PathBuf {
        self.root.join(format!("{id}.json"))
    }

    fn acquire_write_lock(&self, id: DocumentId) -> Result<(File, File), ApplicationError> {
        let migration_path = self.root.join(".migration.lock");
        let migration = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&migration_path)
            .map_err(|error| storage_error(&migration_path, error))?;
        FileExt::lock_shared(&migration).map_err(|error| storage_error(&migration_path, error))?;
        if self.root.join(".migrated").exists() || self.root.join(".migration-pending").exists() {
            return Err(ApplicationError::InvalidConfiguration(
                "document store is fenced for migration and is read-only".into(),
            ));
        }
        let path = self.root.join(format!(".{id}.lock"));
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .map_err(|error| storage_error(&path, error))?;
        file.lock_exclusive()
            .map_err(|error| storage_error(&path, error))?;
        Ok((migration, file))
    }

    fn write_atomic(&self, record: &DocumentRecord) -> Result<(), ApplicationError> {
        let stored = StoredRecord::from(record);
        let target = self.record_path(record.id);
        let mut temporary = tempfile::NamedTempFile::new_in(&self.root)
            .map_err(|error| storage_error(&self.root, error))?;
        serde_json::to_writer(temporary.as_file_mut(), &stored)
            .map_err(|error| storage_error(&target, error))?;
        temporary
            .as_file_mut()
            .write_all(b"\n")
            .map_err(|error| storage_error(&target, error))?;
        temporary
            .as_file_mut()
            .sync_all()
            .map_err(|error| storage_error(&target, error))?;
        temporary
            .persist(&target)
            .map_err(|error| storage_error(&target, error.error))?;
        Ok(())
    }
}

impl DocumentRepository for FileDocumentRepository {
    fn insert(&self, record: DocumentRecord) -> Result<(), ApplicationError> {
        let _lock = self.acquire_write_lock(record.id)?;
        let path = self.record_path(record.id);
        if path.exists() {
            return Err(ApplicationError::DocumentAlreadyExists(
                record.id.to_string(),
            ));
        }
        self.write_atomic(&record)
    }

    fn replace(&self, record: DocumentRecord) -> Result<(), ApplicationError> {
        let _lock = self.acquire_write_lock(record.id)?;
        let stored = self
            .find(record.id)?
            .ok_or_else(|| ApplicationError::DocumentNotFound(record.id.to_string()))?;
        if stored.is_sealed() {
            return Err(ApplicationError::DocumentAlreadySealed(
                record.id.to_string(),
            ));
        }
        if stored.version != record.version
            || stored.name != record.name
            || stored.digest != record.digest
            || stored.vault != record.vault
        {
            return Err(ApplicationError::StoredDocumentInconsistent(format!(
                "replacement changed immutable document fields for {}",
                record.id
            )));
        }
        self.write_atomic(&record)
    }

    fn find(&self, id: DocumentId) -> Result<Option<DocumentRecord>, ApplicationError> {
        let path = self.record_path(id);
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(storage_error(&path, error)),
        };
        let stored: StoredRecord = serde_json::from_reader(BufReader::new(file))
            .map_err(|error| storage_error(&path, error))?;
        let record = stored.into_record()?;
        if record.id != id {
            return Err(ApplicationError::StoredDocumentInconsistent(format!(
                "record {} was loaded from the path for {id}",
                record.id
            )));
        }
        Ok(Some(record))
    }
}

impl From<&DocumentRecord> for StoredRecord {
    fn from(record: &DocumentRecord) -> Self {
        Self {
            id: record.id,
            version: record.version,
            name: record.name.clone(),
            digest: record.digest.to_hex(),
            vault_base64: encode(&record.vault),
            evidence: record.evidence.as_ref().map(StoredEvidence::from),
        }
    }
}

impl StoredRecord {
    fn into_record(self) -> Result<DocumentRecord, ApplicationError> {
        let digest = Sha256Digest::from_hex(&self.digest)?;
        let vault = decode(&self.vault_base64)?;
        let mut record = DocumentRecord::pending(self.id, self.version, self.name, digest, vault)?;
        if let Some(evidence) = self.evidence {
            record.seal(evidence.into_evidence()?)?;
        }
        Ok(record)
    }
}

impl From<&SealedEvidence> for StoredEvidence {
    fn from(evidence: &SealedEvidence) -> Self {
        Self {
            signature_base64: encode(&evidence.signature),
            timestamp_token_base64: encode(&evidence.timestamp_token),
            signer_certificate_base64: encode(&evidence.signer_certificate_pem),
            issuer_certificate_base64: encode(&evidence.issuer_certificate_pem),
            crl_base64: encode(&evidence.crl_pem),
            tsa_chain_base64: evidence.tsa_chain_pem.as_deref().map(encode),
            openssl_version: evidence.openssl_version.clone(),
        }
    }
}

impl StoredEvidence {
    fn into_evidence(self) -> Result<SealedEvidence, ApplicationError> {
        Ok(SealedEvidence {
            signature: decode(&self.signature_base64)?,
            timestamp_token: decode(&self.timestamp_token_base64)?,
            signer_certificate_pem: decode(&self.signer_certificate_base64)?,
            issuer_certificate_pem: decode(&self.issuer_certificate_base64)?,
            crl_pem: decode(&self.crl_base64)?,
            tsa_chain_pem: self
                .tsa_chain_base64
                .map(|value| decode(&value))
                .transpose()?,
            openssl_version: self.openssl_version,
        })
    }
}

fn encode(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}

fn decode(value: &str) -> Result<Vec<u8>, ApplicationError> {
    STANDARD
        .decode(value)
        .map_err(|error| ApplicationError::Port(format!("invalid stored base64: {error}")))
}

fn storage_error(path: &Path, error: impl std::fmt::Display) -> ApplicationError {
    ApplicationError::Port(format!("document storage {}: {error}", path.display()))
}

/// Decodes the legacy storage format without touching the filesystem.
pub fn decode_document_record(bytes: &[u8]) -> Result<DocumentRecord, ApplicationError> {
    let stored: StoredRecord = serde_json::from_slice(bytes)
        .map_err(|error| ApplicationError::StoredDocumentInconsistent(error.to_string()))?;
    stored.into_record()
}

/// Encodes a record without changing its encrypted bytes or captured evidence.
pub fn encode_document_record(record: &DocumentRecord) -> Result<Vec<u8>, ApplicationError> {
    serde_json::to_vec(&StoredRecord::from(record))
        .map_err(|error| ApplicationError::Port(error.to_string()))
}

pub(crate) fn encode_evidence(
    evidence: &SealedEvidence,
) -> Result<serde_json::Value, ApplicationError> {
    serde_json::to_value(StoredEvidence::from(evidence))
        .map_err(|error| ApplicationError::Port(error.to_string()))
}

pub(crate) fn decode_evidence(
    value: serde_json::Value,
) -> Result<SealedEvidence, ApplicationError> {
    serde_json::from_value::<StoredEvidence>(value)
        .map_err(|error| ApplicationError::StoredDocumentInconsistent(error.to_string()))?
        .into_evidence()
}
