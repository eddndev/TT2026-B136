//! PostgreSQL document operations with current case authorization and one audit commit.

mod authorization;
mod content;
mod integrity;
mod integrity_codec;
pub(crate) use integrity::validate_inventory as validate_integrity_inventory;
mod metadata;
mod metadata_storage;
mod mutations;
mod query;
use authorization::{authorize, authorize_document};
mod storage;
pub(crate) use storage::decode_record;

use std::sync::{Mutex, MutexGuard};

use application::documents::{
    CaseDocumentStore, CaseDocumentSummary, CurrentDocumentMetadata, DocumentAction,
    DocumentMetadata, DocumentOverview, DocumentPage, DocumentQuery, DocumentRecord, MetadataPage,
    MetadataQuery, MetadataRevision, VersionPage, VersionQuery, VersionSelection,
};
use application::ApplicationError;
use domain::audit::ChainedEvent;
use domain::cases::CaseId;
use domain::crypto::{DocumentId, DocumentVersion};
use domain::identity::{Permission, UserId};
use postgres::Client;
use time::OffsetDateTime;

use crate::audit_postgres::{append_transaction, begin_audited, load_transaction};
use crate::postgres_actor::active_actor;

/// Durable case-bound records and authorization rechecked before each commit.
pub struct PostgresCaseDocumentStore {
    client: Mutex<Client>,
}

impl PostgresCaseDocumentStore {
    /// Administrative constructor that applies schema for isolated tests.
    pub fn connect(url: &str) -> Result<Self, ApplicationError> {
        Ok(Self {
            client: Mutex::new(crate::postgres::connect(url)?),
        })
    }

    /// Runtime constructor that performs no DDL and validates audit privileges.
    pub fn open(url: &str) -> Result<Self, ApplicationError> {
        Ok(Self {
            client: Mutex::new(crate::postgres::open(url)?),
        })
    }

    fn client(&self) -> Result<MutexGuard<'_, Client>, ApplicationError> {
        self.client
            .lock()
            .map_err(|_| ApplicationError::Port("document database lock poisoned".into()))
    }
}

impl CaseDocumentStore for PostgresCaseDocumentStore {
    fn list(
        &self,
        actor: UserId,
        case: CaseId,
        query: DocumentQuery,
        at: OffsetDateTime,
    ) -> Result<DocumentPage, ApplicationError> {
        let mut client = self.client()?;
        let mut transaction = begin_audited(&mut client)?;
        let principal = authorize(&mut transaction, actor, case, DocumentAction::List)?;
        let page = query::list(&mut transaction, case, query)?;
        append_transaction(
            &mut transaction,
            &principal.email,
            DocumentAction::List.audit_action(),
            &format!("case:{case}:documents"),
            at,
        )?;
        transaction.commit().map_err(storage::port_error)?;
        Ok(page)
    }

    fn get(
        &self,
        actor: UserId,
        case: CaseId,
        id: DocumentId,
        selection: VersionSelection,
        at: OffsetDateTime,
    ) -> Result<CaseDocumentSummary, ApplicationError> {
        let mut client = self.client()?;
        let mut transaction = begin_audited(&mut client)?;
        let principal =
            authorize_document(&mut transaction, actor, case, id, DocumentAction::Read)?;
        let summary = query::get(&mut transaction, case, id, selection)?;
        append_transaction(
            &mut transaction,
            &principal.email,
            DocumentAction::Read.audit_action(),
            &resource(
                case,
                id,
                summary.document.version,
                &summary.document.digest_hex,
            ),
            at,
        )?;
        transaction.commit().map_err(storage::port_error)?;
        Ok(summary)
    }

    fn history(
        &self,
        actor: UserId,
        case: CaseId,
        id: DocumentId,
        query: VersionQuery,
        at: OffsetDateTime,
    ) -> Result<VersionPage, ApplicationError> {
        let mut client = self.client()?;
        let mut transaction = begin_audited(&mut client)?;
        let principal =
            authorize_document(&mut transaction, actor, case, id, DocumentAction::History)?;
        let page = query::history(&mut transaction, case, id, query)?;
        append_transaction(
            &mut transaction,
            &principal.email,
            DocumentAction::History.audit_action(),
            &format!("case:{case}:document:{id}:versions"),
            at,
        )?;
        transaction.commit().map_err(storage::port_error)?;
        Ok(page)
    }

    fn check_access(
        &self,
        actor: UserId,
        case: CaseId,
        action: DocumentAction,
    ) -> Result<(), ApplicationError> {
        let mut client = self.client()?;
        let mut transaction = client.transaction().map_err(storage::port_error)?;
        authorize(&mut transaction, actor, case, action)?;
        if matches!(action, DocumentAction::Upload) {
            crate::postgres_case_status::require_active(&mut transaction, case)?;
        }
        transaction.commit().map_err(storage::port_error)
    }

    fn load(
        &self,
        actor: UserId,
        case: CaseId,
        id: DocumentId,
        selection: VersionSelection,
        action: DocumentAction,
    ) -> Result<DocumentRecord, ApplicationError> {
        let mut client = self.client()?;
        let mut transaction = client.transaction().map_err(storage::port_error)?;
        authorize_document(&mut transaction, actor, case, id, action)?;
        let record = if action == DocumentAction::ReadContent {
            content::load(&mut transaction, case, id, selection)?
        } else {
            storage::load(&mut transaction, case, id, selection)?
        };
        if matches!(
            action,
            DocumentAction::Append | DocumentAction::Seal | DocumentAction::Classify
        ) {
            crate::postgres_case_status::require_active(&mut transaction, case)?;
        }
        transaction.commit().map_err(storage::port_error)?;
        Ok(record)
    }

    fn insert(
        &self,
        actor: UserId,
        case: CaseId,
        record: DocumentRecord,
        at: OffsetDateTime,
    ) -> Result<DocumentOverview, ApplicationError> {
        mutations::insert(self, actor, case, record, None, at)
    }

    fn append(
        &self,
        actor: UserId,
        case: CaseId,
        expected: DocumentVersion,
        record: DocumentRecord,
        at: OffsetDateTime,
    ) -> Result<DocumentOverview, ApplicationError> {
        mutations::append(self, actor, case, expected, record, at)
    }

    fn insert_with_metadata(
        &self,
        actor: UserId,
        case: CaseId,
        record: DocumentRecord,
        metadata: DocumentMetadata,
        at: OffsetDateTime,
    ) -> Result<DocumentOverview, ApplicationError> {
        mutations::insert(self, actor, case, record, Some(metadata), at)
    }

    fn get_metadata(
        &self,
        actor: UserId,
        case: CaseId,
        id: DocumentId,
        at: OffsetDateTime,
    ) -> Result<CurrentDocumentMetadata, ApplicationError> {
        self.read_metadata(actor, case, id, at)
    }

    fn replace_metadata(
        &self,
        actor: UserId,
        case: CaseId,
        id: DocumentId,
        expected: MetadataRevision,
        metadata: DocumentMetadata,
        at: OffsetDateTime,
    ) -> Result<CurrentDocumentMetadata, ApplicationError> {
        self.change_metadata(actor, case, id, expected, metadata, at)
    }

    fn metadata_history(
        &self,
        actor: UserId,
        case: CaseId,
        id: DocumentId,
        query: MetadataQuery,
        at: OffsetDateTime,
    ) -> Result<MetadataPage, ApplicationError> {
        self.read_metadata_history(actor, case, id, query, at)
    }

    fn get_overview(
        &self,
        actor: UserId,
        case: CaseId,
        id: DocumentId,
        at: OffsetDateTime,
    ) -> Result<DocumentOverview, ApplicationError> {
        let mut client = self.client()?;
        let mut transaction = begin_audited(&mut client)?;
        let principal =
            authorize_document(&mut transaction, actor, case, id, DocumentAction::Read)?;
        let content = query::get(&mut transaction, case, id, VersionSelection::Current)?;
        let current_metadata = metadata_storage::current(&mut transaction, id)?;
        append_transaction(
            &mut transaction,
            &principal.email,
            DocumentAction::Read.audit_action(),
            &resource(
                case,
                id,
                content.document.version,
                &content.document.digest_hex,
            ),
            at,
        )?;
        transaction.commit().map_err(storage::port_error)?;
        Ok(DocumentOverview {
            content,
            current_metadata,
        })
    }

    fn seal(
        &self,
        actor: UserId,
        case: CaseId,
        record: DocumentRecord,
        at: OffsetDateTime,
    ) -> Result<(), ApplicationError> {
        let mut client = self.client()?;
        let mut transaction = begin_audited(&mut client)?;
        let principal = authorize_document(
            &mut transaction,
            actor,
            case,
            record.id,
            DocumentAction::Seal,
        )?;
        let current = storage::load(
            &mut transaction,
            case,
            record.id,
            VersionSelection::Exact(record.version),
        )?;
        crate::postgres_case_status::require_active(&mut transaction, case)?;
        if current.is_sealed() {
            return Err(ApplicationError::DocumentAlreadySealed(
                record.id.to_string(),
            ));
        }
        storage::unchanged_document(&current, &record)?;
        let evidence = record.evidence.as_ref().ok_or_else(|| {
            ApplicationError::InvalidInput("seal requires captured evidence".into())
        })?;
        let json = crate::documents::encode_evidence(evidence)?;
        transaction
            .execute(
                "UPDATE documents SET evidence=$1 WHERE id=$2 AND case_id=$3 AND version=$4",
                &[
                    &json,
                    &record.id.as_uuid(),
                    &case.as_uuid(),
                    &i64::from(record.version.get()),
                ],
            )
            .map_err(storage::port_error)?;
        append_transaction(
            &mut transaction,
            &principal.email,
            DocumentAction::Seal.audit_action(),
            &resource(case, record.id, record.version, &record.digest.to_hex()),
            at,
        )?;
        transaction.commit().map_err(storage::port_error)
    }

    fn record_access(
        &self,
        actor: UserId,
        case: CaseId,
        record: &DocumentRecord,
        action: DocumentAction,
        at: OffsetDateTime,
    ) -> Result<(), ApplicationError> {
        content::record_access(self, actor, case, record, action, at)
    }

    fn audit_entries(&self, actor: UserId) -> Result<Vec<ChainedEvent>, ApplicationError> {
        let mut client = self.client()?;
        let mut transaction = client.transaction().map_err(storage::port_error)?;
        let principal = active_actor(&mut transaction, actor)?;
        if !principal.role.allows(Permission::VerifyAudit) {
            return Err(ApplicationError::PermissionDenied);
        }
        let entries = load_transaction(&mut transaction)?;
        transaction.commit().map_err(storage::port_error)?;
        Ok(entries)
    }
}

fn resource(case: CaseId, id: DocumentId, version: DocumentVersion, digest: &str) -> String {
    format!(
        "case:{case}:document:{id}:version:{}:sha256:{digest}",
        version.get()
    )
}
