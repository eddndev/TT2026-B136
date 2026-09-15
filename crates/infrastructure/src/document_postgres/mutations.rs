use application::documents::{
    CaseDocumentSummary, DocumentAction, DocumentMetadata, DocumentOverview, DocumentRecord,
    DocumentSummary, MetadataRevision, VersionSelection,
};
use application::ApplicationError;
use domain::cases::CaseId;
use domain::crypto::DocumentVersion;
use domain::identity::UserId;
use time::OffsetDateTime;

use super::{
    authorization::{authorize, authorize_document},
    metadata_storage, resource, storage, PostgresCaseDocumentStore,
};
use crate::audit_postgres::{append_transaction, begin_audited};

pub(super) fn insert(
    store: &PostgresCaseDocumentStore,
    actor: UserId,
    case: CaseId,
    record: DocumentRecord,
    metadata: Option<DocumentMetadata>,
    at: OffsetDateTime,
) -> Result<DocumentOverview, ApplicationError> {
    if record.version != DocumentVersion::initial() || record.is_sealed() {
        return Err(ApplicationError::InvalidInput(
            "new documents must start at unsealed version one".into(),
        ));
    }
    let mut client = store.client()?;
    let mut transaction = begin_audited(&mut client)?;
    let principal = authorize(&mut transaction, actor, case, DocumentAction::Upload)?;
    if metadata.is_some() && !principal.role.allows(DocumentAction::Classify.permission()) {
        return Err(ApplicationError::PermissionDenied);
    }
    storage::insert(&mut transaction, case, &record)?;
    let current_metadata = match metadata {
        Some(values) => metadata_storage::insert(
            &mut transaction,
            record.id,
            MetadataRevision::new(1),
            values,
            &principal,
            at,
        )?,
        None => metadata_storage::current(&mut transaction, record.id)?,
    };
    append_transaction(
        &mut transaction,
        &principal.email,
        DocumentAction::Upload.audit_action(),
        &resource(case, record.id, record.version, &record.digest.to_hex()),
        at,
    )?;
    if current_metadata.metadata_revision.get() > 0 {
        append_transaction(
            &mut transaction,
            &principal.email,
            DocumentAction::Classify.audit_action(),
            &metadata_storage::resource(case, record.id, &current_metadata),
            at,
        )?;
    }
    let overview = DocumentOverview {
        content: CaseDocumentSummary {
            case_id: case,
            document: DocumentSummary::from(&record),
        },
        current_metadata,
    };
    transaction.commit().map_err(storage::port_error)?;
    Ok(overview)
}

pub(super) fn append(
    store: &PostgresCaseDocumentStore,
    actor: UserId,
    case: CaseId,
    expected: DocumentVersion,
    record: DocumentRecord,
    at: OffsetDateTime,
) -> Result<DocumentOverview, ApplicationError> {
    let mut client = store.client()?;
    let mut transaction = begin_audited(&mut client)?;
    let principal = authorize_document(
        &mut transaction,
        actor,
        case,
        record.id,
        DocumentAction::Append,
    )?;
    let current =
        storage::resolve_version(&mut transaction, case, record.id, VersionSelection::Current)?;
    if current != expected {
        return Err(ApplicationError::DocumentVersionConflict);
    }
    let next = current
        .next()
        .map_err(|_| ApplicationError::DocumentVersionExhausted)?;
    if record.version != next || record.is_sealed() {
        return Err(ApplicationError::InvalidInput(
            "appended records must be the next unsealed version".into(),
        ));
    }
    storage::insert_snapshot(&mut transaction, case, &record)?;
    let current_metadata = metadata_storage::current(&mut transaction, record.id)?;
    append_transaction(
        &mut transaction,
        &principal.email,
        DocumentAction::Append.audit_action(),
        &resource(case, record.id, record.version, &record.digest.to_hex()),
        at,
    )?;
    let overview = DocumentOverview {
        content: CaseDocumentSummary {
            case_id: case,
            document: DocumentSummary::from(&record),
        },
        current_metadata,
    };
    transaction.commit().map_err(storage::port_error)?;
    Ok(overview)
}
