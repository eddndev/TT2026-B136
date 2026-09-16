//! Administrative publication of immutable internal credential trust snapshots.

pub(crate) mod schema;
mod storage;

use std::sync::{Arc, Mutex};

use application::credential_trust::{
    CredentialTrustExpectation, CredentialTrustSnapshot, CredentialTrustStore,
};
use application::ApplicationError;
use domain::clock::Clock;
use domain::crypto::{CredentialFailure, CredentialTrustInspection};
use postgres::{Client, NoTls};
use uuid::Uuid;

pub(crate) use storage::{current, exact};

/// Uses explicitly privileged database credentials; runtime roles only read trust.
pub struct PostgresCredentialTrustStore {
    client: Mutex<Client>,
    clock: Arc<dyn Clock + Send + Sync>,
}

impl PostgresCredentialTrustStore {
    /// Opens existing schema without running migrations or changing grants.
    pub fn open(url: &str, clock: Arc<dyn Clock + Send + Sync>) -> Result<Self, ApplicationError> {
        let mut client = Client::connect(url, NoTls).map_err(storage::port)?;
        schema::validate(&mut client)?;
        schema::validate_inventory(&mut client)?;
        storage::require_publisher(&mut client)?;
        Ok(Self {
            client: Mutex::new(client),
            clock,
        })
    }
}

impl CredentialTrustStore for PostgresCredentialTrustStore {
    fn publish(
        &self,
        expected: CredentialTrustExpectation,
        inspection: CredentialTrustInspection,
    ) -> Result<CredentialTrustSnapshot, ApplicationError> {
        let next = expected
            .next()
            .ok_or(ApplicationError::CredentialTrustRevisionExhausted)?;
        let mut client = self.client.lock().map_err(|_| {
            ApplicationError::Port("credential trust database lock poisoned".into())
        })?;
        let mut transaction = crate::audit_postgres::begin_audited(&mut client)?;
        storage::require_publisher(&mut transaction)?;
        let head = current(&mut transaction)?;
        if head.as_ref().map_or(0, |head| head.revision.get()) != expected.get() {
            return Err(ApplicationError::CredentialTrustRevisionConflict);
        }
        if let Some(previous) = &head {
            if previous.inspection.root_der != inspection.root_der
                || previous.inspection.root_fingerprint != inspection.root_fingerprint
            {
                return Err(ApplicationError::CredentialTrustChanged);
            }
            if inspection.crl_number <= previous.inspection.crl_number
                || inspection.crl_this_update < previous.inspection.crl_this_update
            {
                return Err(ApplicationError::InvalidInput(
                    "published revocation lists must advance their number without moving thisUpdate backwards".into()));
            }
        }
        let at = self.clock.now();
        valid_at(&inspection, at.unix_timestamp())?;
        let deployment = head.map_or_else(Uuid::new_v4, |head| head.deployment_id);
        if expected == CredentialTrustExpectation::Absent {
            transaction.execute("INSERT INTO participant_credential_authority(deployment_id,root_der,root_fingerprint)
                VALUES($1,$2,$3)", &[&deployment,&inspection.root_der,&&inspection.root_fingerprint.as_bytes()[..]])
                .map_err(storage::port)?;
        }
        let published_by: String = transaction
            .query_one("SELECT SESSION_USER", &[])
            .map_err(storage::port)?
            .get(0);
        transaction
            .execute(
                "INSERT INTO participant_credential_trust_revisions(
            deployment_id,revision,crl_der,crl_digest,crl_number,crl_this_update,crl_next_update,
            valid_from,valid_until,published_at_seconds,published_at_nanoseconds,published_by)
            VALUES($1,$2,$3,$4,$5::text::numeric,$6,$7,$8,$9,$10,$11,$12)",
                &[
                    &deployment,
                    &i64::from(next.get()),
                    &inspection.crl_der,
                    &&inspection.crl_digest.as_bytes()[..],
                    &inspection.crl_number.to_string(),
                    &inspection.crl_this_update,
                    &inspection.crl_next_update,
                    &inspection.valid_from,
                    &inspection.valid_until,
                    &at.unix_timestamp(),
                    &(at.nanosecond() as i32),
                    &published_by,
                ],
            )
            .map_err(storage::port)?;
        let result =
            exact(&mut transaction, deployment, next)?.ok_or_else(storage::inconsistent)?;
        crate::audit_postgres::append_transaction(
            &mut transaction,
            &format!("database-admin:{published_by}"),
            "participant.credential_trust_published",
            &format!(
                "credential-trust:{deployment}:revision:{}:root:{}:crl:{}",
                next.get(),
                inspection.root_fingerprint.to_hex(),
                inspection.crl_digest.to_hex()
            ),
            at,
        )?;
        transaction.commit().map_err(storage::port)?;
        Ok(result)
    }
}

fn valid_at(material: &CredentialTrustInspection, at: i64) -> Result<(), ApplicationError> {
    let failure = if at < material.crl_this_update {
        Some(CredentialFailure::CrlNotYetValid)
    } else if at > material.crl_next_update {
        Some(CredentialFailure::CrlExpired)
    } else if at < material.valid_from {
        Some(CredentialFailure::NotYetValid)
    } else if at > material.valid_until {
        Some(CredentialFailure::Expired)
    } else {
        None
    };
    failure.map_or(Ok(()), |failure| Err(failure.into()))
}
