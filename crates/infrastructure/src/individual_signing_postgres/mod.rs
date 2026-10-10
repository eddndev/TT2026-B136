//! Exact individual signature receipts committed with document evidence and audit.
mod inventory;
mod read;
mod write;

use application::{
    document_signing::{
        IndividualSigningStore, SigningCommit, SigningReceipt, SigningVerificationContext,
        SigningWorkflowError, VerifiedDocumentSigning,
    },
    identity::Principal,
    ApplicationError,
};
use domain::{cases::CaseId, clock::Clock, crypto::DocumentVersionRef};
use postgres::Client;
use std::sync::{Arc, Mutex, MutexGuard};
use uuid::Uuid;

pub(crate) use inventory::validate as validate_inventory;

pub struct PostgresIndividualSigningStore {
    client: Mutex<Client>,
    clock: Arc<dyn Clock + Send + Sync>,
}
impl PostgresIndividualSigningStore {
    pub fn open(
        source: &(impl crate::PostgresConnectionSource + ?Sized),
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Result<Self, ApplicationError> {
        let mut client = crate::postgres::open(source)?;
        client
            .query_one(
                "SELECT pg_catalog.set_config('search_path',
            pg_catalog.format('pg_catalog,%I,pg_temp',n.nspname),false)
            FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON n.oid=c.relnamespace
            WHERE c.oid='individual_document_signatures'::pg_catalog.regclass",
                &[],
            )
            .map_err(port)?;
        Ok(Self {
            client: Mutex::new(client),
            clock,
        })
    }
    fn client(&self) -> Result<MutexGuard<'_, Client>, ApplicationError> {
        self.client.lock().map_err(|_| inconsistent())
    }
}
impl IndividualSigningStore for PostgresIndividualSigningStore {
    fn find(
        &self,
        principal: &Principal,
        case: CaseId,
        reference: DocumentVersionRef,
        operation: Uuid,
    ) -> Result<Option<SigningReceipt>, ApplicationError> {
        read::find(self, principal, case, reference, operation)
    }
    fn verification_context(
        &self,
        principal: &Principal,
        case: CaseId,
        reference: DocumentVersionRef,
    ) -> Result<SigningVerificationContext, ApplicationError> {
        read::verification_context(self, principal, case, reference)
    }
    fn commit(&self, verified: VerifiedDocumentSigning) -> Result<SigningCommit, ApplicationError> {
        write::commit(self, verified)
    }
}
fn port(_: postgres::Error) -> ApplicationError {
    ApplicationError::Port("individual document signature database operation failed".into())
}
fn inconsistent() -> ApplicationError {
    SigningWorkflowError::Inconsistent.into()
}
fn resource(receipt: &SigningReceipt) -> String {
    use domain::crypto::DocumentHasher;
    let value = &receipt.consent;
    format!(
        "case:{}:document:{}:version:{}:signer:{}:operation:{}:consent:{}",
        value.case(),
        value.document().id,
        value.document().version.get(),
        value.account().id(),
        value.operation(),
        crate::RingSha256Hasher
            .hash_bytes(&value.canonical_bytes())
            .to_hex()
    )
}
