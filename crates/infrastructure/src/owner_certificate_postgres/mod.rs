//! Audited immutable Owner certificate evidence, without certificate login.
mod account;
mod decode;
mod inventory;
mod query;
mod registration;
mod validation;
mod withdrawal;

use std::sync::{Arc, Mutex, MutexGuard};

use application::{identity::owner_certificates::*, ApplicationError};
use domain::{clock::Clock, identity::UserId};
use postgres::Client;
use uuid::Uuid;

pub(crate) use inventory::validate as validate_inventory;

pub struct PostgresOwnerCertificateStore {
    client: Mutex<Client>,
    clock: Arc<dyn Clock + Send + Sync>,
}

impl PostgresOwnerCertificateStore {
    /// Opens validated runtime state without migrations or privilege changes.
    pub fn open(
        source: &(impl crate::PostgresConnectionSource + ?Sized),
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Result<Self, ApplicationError> {
        let mut client = crate::postgres::open(source)?;
        // Resolve the validated relation, not an unrelated search_path prefix.
        client
            .query_one(
                "SELECT pg_catalog.set_config('search_path',
            pg_catalog.format('pg_catalog,%I,pg_temp',n.nspname),false)
            FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON n.oid=c.relnamespace
            WHERE c.oid='owner_certificate_registrations'::pg_catalog.regclass",
                &[],
            )
            .map_err(port)?;
        Ok(Self {
            client: Mutex::new(client),
            clock,
        })
    }

    fn client(&self) -> Result<MutexGuard<'_, Client>, ApplicationError> {
        self.client.lock().map_err(|_| storage())
    }
}

impl OwnerCertificateStore for PostgresOwnerCertificateStore {
    fn load_registration(
        &self,
        actor: UserId,
    ) -> Result<OwnerRegistrationContext, ApplicationError> {
        self.registration_context(actor)
    }
    fn find(
        &self,
        actor: UserId,
        binding: Uuid,
    ) -> Result<Option<OwnerBindingReceipt>, ApplicationError> {
        self.receipt(actor, binding)
    }
    fn find_current(&self, actor: UserId) -> Result<Option<OwnerBindingReceipt>, ApplicationError> {
        self.current_receipt(actor)
    }
    fn load_withdrawal(
        &self,
        actor: UserId,
        binding: Uuid,
    ) -> Result<OwnerWithdrawalContext, ApplicationError> {
        self.withdrawal_context(actor, binding)
    }
    fn commit_registration(
        &self,
        command: VerifiedOwnerRegistration,
    ) -> Result<OwnerBindingCommit, ApplicationError> {
        self.register(command)
    }
    fn commit_withdrawal(
        &self,
        command: PreparedOwnerWithdrawal,
    ) -> Result<OwnerBindingCommit, ApplicationError> {
        self.withdraw(command)
    }
}

fn port(_: postgres::Error) -> ApplicationError {
    storage()
}
fn storage() -> ApplicationError {
    ApplicationError::Port("owner certificate database operation failed".into())
}
fn inconsistent() -> ApplicationError {
    OwnerCertificateError::Inconsistent.into()
}
