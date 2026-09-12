//! Offline inspection and atomic import of encrypted local records.

mod database;
mod files;
mod reconciliation;
pub use files::require_completed_import;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use application::documents::{validate_record_with_ports, DocumentRecord, DocumentValidationPorts};
use application::ApplicationError;
use domain::audit::{verify_chain, ChainVerification, ChainedEvent, GENESIS_PREVIOUS};
use domain::cases::CaseId;
use domain::crypto::{DocumentHasher, DocumentId};
use serde::{Deserialize, Serialize};

use crate::audit::decode_audit_snapshot;
use crate::documents::decode_document_record;
use crate::{
    EnvelopeKeyManager, Rfc3161Verifier, RingAesGcmCipher, RingSha256Hasher, RsaPkcs1Verifier,
    X509ChainValidator,
};

/// Non-secret receipt used for reconciliation and restart after an interrupted import.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportReport {
    pub fingerprint: String,
    pub documents: usize,
    pub sealed_documents: usize,
    pub audit_entries: usize,
    pub audit_head: String,
    pub source_files: BTreeMap<String, String>,
    pub assignments: BTreeMap<String, String>,
    pub mapping_hash: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Mapping {
    documents: Vec<Assignment>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Assignment {
    document_id: DocumentId,
    case_id: CaseId,
}

/// A fully validated snapshot; inspection does not create or modify files.
pub struct LegacyImport {
    root: PathBuf,
    mapping: PathBuf,
    mapping_hash: String,
    records: Vec<(CaseId, DocumentRecord)>,
    entries: Vec<ChainedEvent>,
    report: ImportReport,
}

impl LegacyImport {
    pub fn inspect(root: &Path, mapping: &Path, kek: &[u8]) -> Result<Self, ApplicationError> {
        let mapping_bytes = files::read_regular(mapping)?;
        let assignments: Mapping = serde_json::from_slice(&mapping_bytes).map_err(invalid)?;
        let mut by_id = BTreeMap::new();
        for assignment in assignments.documents {
            if by_id
                .insert(assignment.document_id.to_string(), assignment.case_id)
                .is_some()
            {
                return Err(invalid("duplicate document mapping"));
            }
        }
        let source = files::snapshot(root)?;
        let mut records = Vec::new();
        for (name, bytes) in &source {
            if name == "audit.jsonl" {
                continue;
            }
            let record = decode_document_record(bytes)?;
            if *name != format!("documents/{}.json", record.id) {
                return Err(invalid("document filename and stored identity disagree"));
            }
            let case_id = by_id
                .remove(&record.id.to_string())
                .ok_or_else(|| invalid("document has no explicit case mapping"))?;
            validate_record_with_ports(
                &record,
                kek,
                &DocumentValidationPorts {
                    cipher: &RingAesGcmCipher::new(),
                    keys: &EnvelopeKeyManager::new(),
                    hasher: &RingSha256Hasher::new(),
                    signature_verifier: &RsaPkcs1Verifier::new(),
                    certificate_validator: &X509ChainValidator::new(),
                    timestamp_verifier: &Rfc3161Verifier::new(),
                },
            )?;
            records.push((case_id, record));
        }
        if !by_id.is_empty() {
            return Err(invalid("mapping refers to absent documents"));
        }
        let entries = decode_audit_snapshot(
            source
                .get("audit.jsonl")
                .map(Vec::as_slice)
                .unwrap_or_default(),
        )?;
        if !matches!(
            verify_chain(&RingSha256Hasher::new(), &entries)?,
            ChainVerification::Valid { .. }
        ) || entries
            .iter()
            .enumerate()
            .any(|(index, entry)| entry.event.sequence != index as u64)
        {
            return Err(invalid(
                "legacy audit chain is broken or has a sequence gap",
            ));
        }
        for entry in &entries {
            if let Some(id) = entry.event.resource.strip_prefix("document:") {
                if !records
                    .iter()
                    .any(|(_, record)| record.id.to_string() == id)
                {
                    return Err(invalid("audit history references an absent document"));
                }
            }
        }
        let source_files = files::hashes(&source);
        let mapping_hash = digest(&mapping_bytes);
        let assignments: BTreeMap<String, String> = records
            .iter()
            .map(|(case, record)| (record.id.to_string(), case.to_string()))
            .collect();
        let fingerprint = digest(
            &serde_json::to_vec(&(&source_files, &mapping_hash, &assignments)).map_err(invalid)?,
        );
        let report = ImportReport {
            fingerprint,
            documents: records.len(),
            sealed_documents: records
                .iter()
                .filter(|(_, record)| record.is_sealed())
                .count(),
            audit_entries: entries.len(),
            audit_head: entries
                .last()
                .map(|entry| entry.chain)
                .unwrap_or(GENESIS_PREVIOUS)
                .to_hex(),
            source_files,
            assignments,
            mapping_hash: mapping_hash.clone(),
        };
        Ok(Self {
            root: root.into(),
            mapping: mapping.into(),
            mapping_hash,
            records,
            entries,
            report,
        })
    }

    pub fn report(&self) -> &ImportReport {
        &self.report
    }
}

fn invalid(error: impl std::fmt::Display) -> ApplicationError {
    ApplicationError::InvalidInput(format!("legacy import: {error}"))
}

fn digest(bytes: &[u8]) -> String {
    RingSha256Hasher::new().hash_bytes(bytes).to_hex()
}
