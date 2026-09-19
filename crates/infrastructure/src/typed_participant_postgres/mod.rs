//! Transactional represented identities, typed roles and their captured provenance.
mod authorization;
mod candidates;
mod commit;
mod credentials;
mod directory;
mod documents;
mod port_impl;
mod preparation;
mod query;
mod review;
pub(crate) mod storage;
mod write;

use crate::audit_postgres::{append_transaction, begin_audited};
use application::{identity::Principal, typed_participants::*};
use authorization::authorize;
use domain::{
    cases::CaseId,
    clock::Clock,
    crypto::{DocumentHasher, Sha256Digest},
    identity::UserId,
};
use postgres::{Client, GenericClient, Transaction};
use std::sync::{Arc, Mutex, MutexGuard};
use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};

pub struct PostgresTypedParticipantStore {
    client: Mutex<Client>,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
    clock: Arc<dyn Clock + Send + Sync>,
}
impl PostgresTypedParticipantStore {
    pub fn open(
        url: &str,
        hasher: Arc<dyn DocumentHasher + Send + Sync>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Result<Self, ApplicationError> {
        Ok(Self {
            client: Mutex::new(crate::postgres::open(url)?),
            hasher,
            clock,
        })
    }
    pub fn connect(
        url: &str,
        hasher: Arc<dyn DocumentHasher + Send + Sync>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Result<Self, ApplicationError> {
        Ok(Self {
            client: Mutex::new(crate::postgres::connect(url)?),
            hasher,
            clock,
        })
    }
    fn client(&self) -> Result<MutexGuard<'_, Client>, ApplicationError> {
        self.client.lock().map_err(|_| inconsistent())
    }
}
fn port(error: postgres::Error) -> ApplicationError {
    let message = "typed participant database operation failed";
    match crate::postgres_port::error(message, error) {
        ApplicationError::ClassifiedPort { kind, .. } => ApplicationError::ClassifiedPort {
            kind,
            message: message.into(),
        },
        _ => ApplicationError::Port(message.into()),
    }
}
fn inconsistent() -> ApplicationError {
    ApplicationError::StoredParticipantInconsistent(
        "stored typed participant snapshot is inconsistent".into(),
    )
}
fn digest(bytes: Vec<u8>) -> Result<Sha256Digest, ApplicationError> {
    Sha256Digest::from_bytes(&bytes).map_err(|_| inconsistent())
}
fn revision(value: i64) -> Result<ParticipantRevision, ApplicationError> {
    ParticipantRevision::new(u32::try_from(value).map_err(|_| inconsistent())?)
        .map_err(|_| inconsistent())
}
fn subject_revision(value: i64) -> Result<SubjectRevision, ApplicationError> {
    SubjectRevision::new(u32::try_from(value).map_err(|_| inconsistent())?)
        .map_err(|_| inconsistent())
}
fn timestamp(value: &str) -> Result<OffsetDateTime, ApplicationError> {
    let at = OffsetDateTime::parse(value, &Rfc3339).map_err(|_| inconsistent())?;
    if canonical_time(at)? != value {
        return Err(inconsistent());
    }
    Ok(at)
}
fn canonical_time(at: OffsetDateTime) -> Result<String, ApplicationError> {
    at.to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .map_err(|_| inconsistent())
}
fn actor(principal: &Principal) -> ParticipantActorSnapshot {
    ParticipantActorSnapshot {
        id: principal.id,
        email: principal.email.clone(),
    }
}
fn captured_actor(row: &postgres::Row) -> Result<ParticipantActorSnapshot, ApplicationError> {
    let email: String = row
        .try_get("changed_by_email")
        .map_err(|_| inconsistent())?;
    if email.is_empty() || email.trim() != email || email.chars().any(char::is_control) {
        return Err(inconsistent());
    }
    Ok(ParticipantActorSnapshot {
        id: UserId::from_uuid(row.try_get("changed_by").map_err(|_| inconsistent())?),
        email,
    })
}
fn audit(
    tx: &mut Transaction<'_>,
    principal: &Principal,
    action: &str,
    resource: &str,
    at: OffsetDateTime,
) -> Result<(), ApplicationError> {
    append_transaction(tx, &principal.email, action, resource, at)?;
    Ok(())
}
