//! PostgreSQL document operations with current case authorization and one audit commit.

mod storage;
pub(crate) use storage::decode_record;

use std::sync::{Mutex, MutexGuard};

use application::documents::{CaseDocumentStore, DocumentAction, DocumentRecord};
use application::identity::Principal;
use application::ApplicationError;
use domain::audit::ChainedEvent;
use domain::cases::CaseId;
use domain::crypto::DocumentId;
use domain::identity::{Permission, Role, UserId};
use postgres::{Client, Transaction};
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
    fn check_access(
        &self,
        actor: UserId,
        case: CaseId,
        action: DocumentAction,
    ) -> Result<(), ApplicationError> {
        let mut client = self.client()?;
        let mut transaction = client.transaction().map_err(storage::port_error)?;
        authorize(&mut transaction, actor, case, action)?;
        transaction.commit().map_err(storage::port_error)
    }

    fn load(
        &self,
        actor: UserId,
        case: CaseId,
        id: DocumentId,
        action: DocumentAction,
    ) -> Result<DocumentRecord, ApplicationError> {
        let mut client = self.client()?;
        let mut transaction = client.transaction().map_err(storage::port_error)?;
        authorize_document(&mut transaction, actor, case, id, action)?;
        let record = storage::load(&mut transaction, case, id)?;
        transaction.commit().map_err(storage::port_error)?;
        Ok(record)
    }

    fn insert(
        &self,
        actor: UserId,
        case: CaseId,
        record: DocumentRecord,
        at: OffsetDateTime,
    ) -> Result<(), ApplicationError> {
        if record.is_sealed() {
            return Err(ApplicationError::InvalidInput(
                "uploaded records must not contain sealed evidence".into(),
            ));
        }
        let mut client = self.client()?;
        let mut transaction = begin_audited(&mut client)?;
        let principal = authorize(&mut transaction, actor, case, DocumentAction::Upload)?;
        storage::insert(&mut transaction, case, &record)?;
        append_transaction(
            &mut transaction,
            &principal.email,
            DocumentAction::Upload.audit_action(),
            &resource(case, record.id),
            at,
        )?;
        transaction.commit().map_err(storage::port_error)
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
        let current = storage::load(&mut transaction, case, record.id)?;
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
                "UPDATE documents SET evidence=$1 WHERE id=$2 AND case_id=$3",
                &[&json, &record.id.as_uuid(), &case.as_uuid()],
            )
            .map_err(storage::port_error)?;
        append_transaction(
            &mut transaction,
            &principal.email,
            DocumentAction::Seal.audit_action(),
            &resource(case, record.id),
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
        if !matches!(action, DocumentAction::Verify | DocumentAction::Export) {
            return Err(ApplicationError::InvalidInput(
                "access events require verification or export".into(),
            ));
        }
        let mut client = self.client()?;
        let mut transaction = begin_audited(&mut client)?;
        let principal = authorize_document(&mut transaction, actor, case, record.id, action)?;
        let current = storage::load(&mut transaction, case, record.id)?;
        storage::unchanged_document(&current, record)?;
        if current.evidence != record.evidence {
            return Err(ApplicationError::ConcurrentModification);
        }
        append_transaction(
            &mut transaction,
            &principal.email,
            action.audit_action(),
            &resource(case, record.id),
            at,
        )?;
        transaction.commit().map_err(storage::port_error)
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

fn authorize(
    transaction: &mut Transaction<'_>,
    actor: UserId,
    case: CaseId,
    action: DocumentAction,
) -> Result<Principal, ApplicationError> {
    let principal = active_actor(transaction, actor)?;
    if !principal.role.allows(action.permission()) {
        return Err(ApplicationError::PermissionDenied);
    }
    let visible = if principal.role == Role::Owner {
        transaction.query_opt("SELECT id FROM cases WHERE id=$1", &[&case.as_uuid()])
    } else {
        transaction.query_opt(
            "SELECT m.case_id FROM case_memberships m WHERE m.case_id=$1 AND m.user_id=$2 FOR SHARE",
            &[&case.as_uuid(), &actor.as_uuid()],
        )
    }.map_err(storage::port_error)?;
    visible.ok_or(ApplicationError::CaseNotFound)?;
    Ok(principal)
}

fn authorize_document(
    transaction: &mut Transaction<'_>,
    actor: UserId,
    case: CaseId,
    id: DocumentId,
    action: DocumentAction,
) -> Result<Principal, ApplicationError> {
    authorize(transaction, actor, case, action).map_err(|error| match error {
        ApplicationError::CaseNotFound => ApplicationError::DocumentNotFound(id.to_string()),
        other => other,
    })
}

fn resource(case: CaseId, document: DocumentId) -> String {
    format!("case:{case}:document:{document}")
}
