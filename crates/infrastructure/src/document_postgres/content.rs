use application::document_content::MAX_DOCUMENT_CONTENT_BYTES;
use application::document_integrity::DocumentIntegrityFailure;
use application::documents::{DocumentAction, DocumentRecord, VersionSelection};
use application::vault::VaultReadLimits;
use application::ApplicationError;
use domain::{cases::CaseId, crypto::DocumentId, identity::UserId};
use postgres::Transaction;
use time::OffsetDateTime;

use super::storage::{decode_record, port_error, resolve_version};
use super::{authorization::authorize_document, resource, storage, PostgresCaseDocumentStore};
use crate::audit_postgres::{append_transaction, begin_audited};

/// Bounds the database transfer before materializing an encrypted content snapshot.
/// Captured signing evidence is unrelated to content delivery and stays unloaded.
pub(super) fn load(
    tx: &mut Transaction<'_>,
    case: CaseId,
    id: DocumentId,
    selection: VersionSelection,
) -> Result<DocumentRecord, ApplicationError> {
    let version = resolve_version(tx, case, id, selection)?;
    let version = i64::from(version.get());
    let maximum = VaultReadLimits::aes256_gcm(MAX_DOCUMENT_CONTENT_BYTES)?.max_vault_bytes() as i64;
    let probe = tx.query_opt(
        "SELECT octet_length(vault)::bigint FROM documents WHERE id=$1 AND case_id=$2 AND version=$3",
        &[&id.as_uuid(), &case.as_uuid(), &version],
    ).map_err(port_error)?.ok_or_else(|| ApplicationError::DocumentNotFound(id.to_string()))?;
    if probe.get::<_, i64>(0) > maximum {
        return Err(ApplicationError::DocumentContentTooLarge);
    }
    let row = tx
        .query_opt(
            "SELECT id,version,name,digest,vault,NULL::jsonb AS evidence FROM documents
         WHERE id=$1 AND case_id=$2 AND version=$3 AND octet_length(vault)::bigint<=$4",
            &[&id.as_uuid(), &case.as_uuid(), &version, &maximum],
        )
        .map_err(port_error)?
        .ok_or(ApplicationError::DocumentContentTooLarge)?;
    decode_record(row)
}

pub(super) fn record_access(
    store: &PostgresCaseDocumentStore,
    actor: UserId,
    case: CaseId,
    record: &DocumentRecord,
    action: DocumentAction,
    at: OffsetDateTime,
) -> Result<(), ApplicationError> {
    if !matches!(
        action,
        DocumentAction::Verify | DocumentAction::Export | DocumentAction::ReadContent
    ) {
        return Err(ApplicationError::InvalidInput(
            "access events require content, verification or export".into(),
        ));
    }
    let mut client = store.client()?;
    let mut tx = begin_audited(&mut client)?;
    let principal = authorize_document(&mut tx, actor, case, record.id, action)?;
    let selection = VersionSelection::Exact(record.version);
    let current = if action == DocumentAction::ReadContent {
        load(&mut tx, case, record.id, selection).map_err(|error| {
            if matches!(error, ApplicationError::DocumentContentTooLarge) {
                changed()
            } else {
                error
            }
        })?
    } else {
        storage::load(&mut tx, case, record.id, selection)?
    };
    storage::unchanged_document(&current, record).map_err(|error| {
        if action == DocumentAction::ReadContent {
            changed()
        } else {
            error
        }
    })?;
    if action != DocumentAction::ReadContent && current.evidence != record.evidence {
        return Err(ApplicationError::ConcurrentModification);
    }
    let at = if action == DocumentAction::ReadContent {
        OffsetDateTime::now_utc()
    } else {
        at
    };
    append_transaction(
        &mut tx,
        &principal.email,
        action.audit_action(),
        &resource(case, record.id, record.version, &record.digest.to_hex()),
        at,
    )?;
    tx.commit().map_err(port_error)
}

fn changed() -> ApplicationError {
    ApplicationError::DocumentContentValidationFailed(DocumentIntegrityFailure::SnapshotChanged)
}
