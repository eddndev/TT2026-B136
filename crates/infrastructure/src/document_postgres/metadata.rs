use application::documents::{
    CurrentDocumentMetadata, DocumentAction, DocumentMetadata, MetadataPage, MetadataQuery,
    MetadataRevision,
};
use application::ApplicationError;
use domain::cases::CaseId;
use domain::crypto::DocumentId;
use domain::identity::UserId;
use time::OffsetDateTime;

use super::{
    authorization::authorize_document, metadata_storage, storage, PostgresCaseDocumentStore,
};
use crate::audit_postgres::{append_transaction, begin_audited};

impl PostgresCaseDocumentStore {
    pub(super) fn read_metadata(
        &self,
        actor: UserId,
        case: CaseId,
        id: DocumentId,
        at: OffsetDateTime,
    ) -> Result<CurrentDocumentMetadata, ApplicationError> {
        let mut client = self.client()?;
        let mut transaction = begin_audited(&mut client)?;
        let principal = authorize_document(
            &mut transaction,
            actor,
            case,
            id,
            DocumentAction::ReadMetadata,
        )?;
        metadata_storage::require_document(&mut transaction, case, id)?;
        let current = metadata_storage::current(&mut transaction, id)?;
        append_transaction(
            &mut transaction,
            &principal.email,
            DocumentAction::ReadMetadata.audit_action(),
            &metadata_storage::resource(case, id, &current),
            at,
        )?;
        transaction.commit().map_err(storage::port_error)?;
        Ok(current)
    }

    pub(super) fn change_metadata(
        &self,
        actor: UserId,
        case: CaseId,
        id: DocumentId,
        expected: MetadataRevision,
        values: DocumentMetadata,
        at: OffsetDateTime,
    ) -> Result<CurrentDocumentMetadata, ApplicationError> {
        let mut client = self.client()?;
        let mut transaction = begin_audited(&mut client)?;
        let principal =
            authorize_document(&mut transaction, actor, case, id, DocumentAction::Classify)?;
        metadata_storage::require_document(&mut transaction, case, id)?;
        let current = metadata_storage::current(&mut transaction, id)?;
        if current.metadata_revision != expected {
            return Err(ApplicationError::DocumentMetadataConflict);
        }
        let next = expected
            .next()
            .ok_or(ApplicationError::DocumentMetadataRevisionExhausted)?;
        let current = metadata_storage::insert(&mut transaction, id, next, values, &principal, at)?;
        append_transaction(
            &mut transaction,
            &principal.email,
            DocumentAction::Classify.audit_action(),
            &metadata_storage::resource(case, id, &current),
            at,
        )?;
        transaction.commit().map_err(storage::port_error)?;
        Ok(current)
    }

    pub(super) fn read_metadata_history(
        &self,
        actor: UserId,
        case: CaseId,
        id: DocumentId,
        query: MetadataQuery,
        at: OffsetDateTime,
    ) -> Result<MetadataPage, ApplicationError> {
        let mut client = self.client()?;
        let mut transaction = begin_audited(&mut client)?;
        let principal = authorize_document(
            &mut transaction,
            actor,
            case,
            id,
            DocumentAction::MetadataHistory,
        )?;
        metadata_storage::require_document(&mut transaction, case, id)?;
        let before = query.before_revision().map(|r| i64::from(r.get()));
        let rows = transaction.query(
            "SELECT metadata_revision,document_type,classification,tags,metadata_digest,changed_at,changed_by,changed_by_email
             FROM document_metadata_revisions WHERE document_id=$1 AND ($2::bigint IS NULL OR metadata_revision<$2)
             ORDER BY metadata_revision DESC LIMIT $3",&[&id.as_uuid(),&before,&(i64::from(query.limit())+1)],
        ).map_err(storage::port_error)?;
        let mut revisions = rows
            .iter()
            .map(metadata_storage::decode)
            .collect::<Result<Vec<_>, _>>()?;
        let has_more = revisions.len() > query.limit() as usize;
        revisions.truncate(query.limit() as usize);
        let next_before_revision = if has_more {
            revisions.last().map(|r| r.metadata_revision)
        } else {
            None
        };
        append_transaction(
            &mut transaction,
            &principal.email,
            DocumentAction::MetadataHistory.audit_action(),
            &format!("case:{case}:document:{id}:metadata-history"),
            at,
        )?;
        transaction.commit().map_err(storage::port_error)?;
        Ok(MetadataPage {
            revisions,
            has_more,
            next_before_revision,
        })
    }
}
