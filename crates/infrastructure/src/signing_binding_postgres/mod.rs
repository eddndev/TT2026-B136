//! Immutable personal signing bindings, reauthorized under the shared audit lock.
mod account;
mod decode;
mod inventory;
mod read;
mod withdrawal;
mod write;

pub(crate) use account::account;
pub(crate) use inventory::validate as validate_inventory;
pub(crate) use read::load;

use application::{document_signing::*, identity::Principal, ApplicationError};
use domain::{
    clock::{Clock, OffsetDateTime},
    document_signing::SignerAccount,
    identity::UserId,
};
use postgres::Client;
use std::sync::{Arc, Mutex, MutexGuard};
use uuid::Uuid;

pub struct PostgresSigningBindingStore {
    client: Mutex<Client>,
    clock: Arc<dyn Clock + Send + Sync>,
}
impl PostgresSigningBindingStore {
    /// Opens validated runtime state without modifying its schema or privileges.
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
            WHERE c.oid='document_signing_bindings'::pg_catalog.regclass",
                &[],
            )
            .map_err(port)?;
        Ok(Self {
            client: Mutex::new(client),
            clock,
        })
    }
    pub(crate) fn client(&self) -> Result<MutexGuard<'_, Client>, ApplicationError> {
        self.client.lock().map_err(|_| storage())
    }
}
impl SigningBindingStore for PostgresSigningBindingStore {
    fn load_context(&self, actor: UserId) -> Result<SigningAccountContext, ApplicationError> {
        self.context(actor)
    }
    fn find(
        &self,
        actor: UserId,
        binding: Uuid,
    ) -> Result<Option<SigningBindingReceipt>, ApplicationError> {
        self.find_receipt(actor, binding)
    }
    fn find_current(
        &self,
        actor: UserId,
    ) -> Result<Option<SigningBindingReceipt>, ApplicationError> {
        self.current_receipt(actor)
    }
    fn commit_binding(
        &self,
        command: VerifiedSigningBinding,
    ) -> Result<SigningBindingReceipt, ApplicationError> {
        self.register(command)
    }
    fn withdraw(
        &self,
        principal: &Principal,
        binding: Uuid,
        expected: SignerAccount,
        at: OffsetDateTime,
    ) -> Result<SigningBindingReceipt, ApplicationError> {
        self.withdraw_binding(principal, binding, expected, at)
    }
}
fn port(_: postgres::Error) -> ApplicationError {
    storage()
}
fn storage() -> ApplicationError {
    ApplicationError::Port("individual signing database operation failed".into())
}
fn inconsistent() -> ApplicationError {
    SigningWorkflowError::Inconsistent.into()
}

fn resource(receipt: &SigningBindingReceipt, withdrawal: bool) -> String {
    let revision = if withdrawal { 2 } else { 1 };
    format!(
        "document-signer:{}:signer:{}:revision:{revision}:statement:{}",
        receipt.statement.material().binding(),
        receipt.statement.account().id(),
        receipt.check.statement_digest.to_hex()
    )
}
