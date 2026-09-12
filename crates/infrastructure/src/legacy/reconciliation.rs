use std::collections::BTreeMap;

use application::documents::DocumentRecord;
use application::ApplicationError;
use domain::audit::{verify_chain, ChainVerification, ChainedEvent, GENESIS_PREVIOUS};
use domain::cases::CaseId;
use postgres::GenericClient;

use crate::audit::decode_audit_snapshot;
use crate::audit_postgres::load_transaction;
use crate::documents::decode_document_record;

use super::{digest, invalid, ImportReport};

pub(super) fn reconcile_source<C: GenericClient>(
    client: &mut C,
    report: &ImportReport,
    source: &BTreeMap<String, Vec<u8>>,
) -> Result<(), ApplicationError> {
    let fingerprint = digest(
        &serde_json::to_vec(&(
            &report.source_files,
            &report.mapping_hash,
            &report.assignments,
        ))
        .map_err(invalid)?,
    );
    if fingerprint != report.fingerprint {
        return Err(invalid(
            "import marker fingerprint disagrees with its source and assignments",
        ));
    }
    let mut records = Vec::new();
    for (name, bytes) in source {
        if name == "audit.jsonl" {
            continue;
        }
        let record = decode_document_record(bytes)?;
        if *name != format!("documents/{}.json", record.id) {
            return Err(invalid("document filename and stored identity disagree"));
        }
        let case = report
            .assignments
            .get(&record.id.to_string())
            .ok_or_else(|| invalid("import marker has no document assignment"))?
            .parse::<uuid::Uuid>()
            .map(CaseId::from_uuid)
            .map_err(invalid)?;
        records.push((case, record));
    }
    let entries = decode_audit_snapshot(
        source
            .get("audit.jsonl")
            .map(Vec::as_slice)
            .unwrap_or_default(),
    )?;
    let head = entries
        .last()
        .map(|entry| entry.chain)
        .unwrap_or(GENESIS_PREVIOUS)
        .to_hex();
    if records.len() != report.documents
        || report.assignments.len() != records.len()
        || records
            .iter()
            .filter(|(_, record)| record.is_sealed())
            .count()
            != report.sealed_documents
        || entries.len() != report.audit_entries
        || head != report.audit_head
    {
        return Err(invalid(
            "import marker counts or historical audit head disagree with source",
        ));
    }
    reconcile(client, &records, &entries, &report.fingerprint)
}

pub(super) fn reconcile<C: GenericClient>(
    client: &mut C,
    records: &[(CaseId, DocumentRecord)],
    original_entries: &[ChainedEvent],
    fingerprint: &str,
) -> Result<(), ApplicationError> {
    let entries = load_transaction(client)?;
    if !matches!(
        verify_chain(&crate::RingSha256Hasher::new(), &entries)?,
        ChainVerification::Valid { .. }
    ) || entries
        .iter()
        .enumerate()
        .any(|(index, entry)| entry.event.sequence != index as u64)
    {
        return Err(invalid(
            "destination audit chain is broken or has a sequence gap",
        ));
    }
    if entries.len() <= original_entries.len()
        || entries[..original_entries.len()] != *original_entries
    {
        return Err(invalid(
            "destination no longer contains the exact legacy audit prefix",
        ));
    }
    let imported = &entries[original_entries.len()].event;
    if imported.actor != "migration"
        || imported.action != "migration.imported"
        || imported.resource != fingerprint
    {
        return Err(invalid("import receipt is missing from audit history"));
    }
    for (case, original) in records {
        let row = client
            .query_opt(
                "SELECT id,case_id,version,name,digest,vault,evidence FROM documents WHERE id=$1",
                &[&original.id.as_uuid()],
            )
            .map_err(invalid)?
            .ok_or_else(|| invalid("imported document is missing"))?;
        if row.get::<_, uuid::Uuid>("case_id") != case.as_uuid() {
            return Err(invalid("imported document case changed"));
        }
        let current = crate::document_postgres::decode_record(row)?;
        if !same_original(original, &current) {
            return Err(invalid(
                "imported encrypted content or captured evidence changed",
            ));
        }
    }
    Ok(())
}

fn same_original(original: &DocumentRecord, current: &DocumentRecord) -> bool {
    original.id == current.id
        && original.version == current.version
        && original.name == current.name
        && original.digest == current.digest
        && original.vault == current.vault
        && (original.evidence.is_none() || original.evidence == current.evidence)
}
