//! Runtime read of the newest credential trust snapshot for the evidence validator.

use std::sync::Mutex;

use application::credential_trust::CredentialTrustSnapshot;
use application::evidence_trust::CredentialTrustReader;
use application::ApplicationError;
use postgres::Client;

use super::storage;

/// Reads trust with runtime privileges inside a read-only transaction.
pub struct PostgresCredentialTrustReader {
    client: Mutex<Client>,
}

impl PostgresCredentialTrustReader {
    /// Opens validated runtime state without migrations or privilege changes.
    pub fn open(
        source: &(impl crate::PostgresConnectionSource + ?Sized),
    ) -> Result<Self, ApplicationError> {
        let mut client = crate::postgres::open(source)?;
        // Resolve the validated relation, not an unrelated search_path prefix.
        client
            .query_one(
                "SELECT pg_catalog.set_config('search_path',
            pg_catalog.format('pg_catalog,%I,pg_temp',n.nspname),false)
            FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON n.oid=c.relnamespace
            WHERE c.oid='participant_credential_trust_revisions'::pg_catalog.regclass",
                &[],
            )
            .map_err(storage::port)?;
        Ok(Self {
            client: Mutex::new(client),
        })
    }
}

impl CredentialTrustReader for PostgresCredentialTrustReader {
    fn current(&self) -> Result<Option<CredentialTrustSnapshot>, ApplicationError> {
        let mut client = self.client.lock().map_err(|_| {
            ApplicationError::Port("credential trust database lock poisoned".into())
        })?;
        let mut transaction = client
            .build_transaction()
            .read_only(true)
            .start()
            .map_err(storage::port)?;
        let snapshot = storage::current(&mut transaction)?;
        transaction.commit().map_err(storage::port)?;
        Ok(snapshot)
    }
}
