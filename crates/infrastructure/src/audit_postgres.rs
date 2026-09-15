//! One PostgreSQL audit chain shared by all backend mutation transactions.

use std::sync::Mutex;

use application::ApplicationError;
use domain::audit::{chain_digest, AuditEvent, AuditLog, ChainedEvent, GENESIS_PREVIOUS};
use domain::crypto::Sha256Digest;
use domain::DomainError;
use postgres::{Client, GenericClient, Row, Transaction};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::RingSha256Hasher;

// Acquired before any mutation or permission check in a committing transaction.
const AUDITED_MUTATION_LOCK: i64 = 0x4155444954;

/// PostgreSQL audit writer for events without another durable mutation.
pub struct PostgresAuditLog {
    client: Mutex<Client>,
}

impl PostgresAuditLog {
    /// Administrative connection that applies schema; useful for isolated tests.
    pub fn connect(url: &str) -> Result<Self, ApplicationError> {
        Ok(Self {
            client: Mutex::new(crate::postgres::connect(url)?),
        })
    }

    /// Runtime connection with immutable audit history privileges and no DDL.
    pub fn open(url: &str) -> Result<Self, ApplicationError> {
        Ok(Self {
            client: Mutex::new(crate::postgres::open(url)?),
        })
    }
}

pub(crate) fn begin_audited(client: &mut Client) -> Result<Transaction<'_>, ApplicationError> {
    let mut transaction = client
        .build_transaction()
        .isolation_level(postgres::IsolationLevel::ReadCommitted)
        .start()
        .map_err(port_error)?;
    lock_mutations(&mut transaction)?;
    Ok(transaction)
}

fn lock_mutations(transaction: &mut Transaction<'_>) -> Result<(), ApplicationError> {
    transaction
        .query_one(
            "SELECT pg_advisory_xact_lock($1)",
            &[&AUDITED_MUTATION_LOCK],
        )
        .map_err(port_error)?;
    Ok(())
}

pub(crate) fn append_transaction(
    transaction: &mut Transaction<'_>,
    actor: &str,
    action: &str,
    resource: &str,
    timestamp: OffsetDateTime,
) -> Result<ChainedEvent, ApplicationError> {
    lock_mutations(transaction)?;
    let last = transaction
        .query_opt(
            "SELECT sequence, chain FROM audit_events ORDER BY sequence DESC LIMIT 1",
            &[],
        )
        .map_err(port_error)?;
    let (sequence, previous) = if let Some(row) = last {
        let sequence: i64 = row.get(0);
        let next = sequence
            .checked_add(1)
            .ok_or_else(|| ApplicationError::Port("audit sequence exhausted".into()))?;
        let bytes: Vec<u8> = row.get(1);
        (next, digest(bytes)?)
    } else {
        (0, GENESIS_PREVIOUS)
    };
    let event = AuditEvent::new(sequence as u64, timestamp, actor, action, resource);
    let chain = chain_digest(&RingSha256Hasher::new(), &previous, &event)?;
    let timestamp = event.timestamp_rfc3339()?;
    transaction.execute(
        "INSERT INTO audit_events(sequence,timestamp,actor,action,resource,chain) VALUES($1,$2,$3,$4,$5,$6)",
        &[&sequence, &timestamp, &actor, &action, &resource, &&chain.as_bytes()[..]],
    ).map_err(port_error)?;
    Ok(ChainedEvent { event, chain })
}

pub(crate) fn load_transaction<C: GenericClient>(
    client: &mut C,
) -> Result<Vec<ChainedEvent>, ApplicationError> {
    client.query("SELECT sequence,timestamp,actor,action,resource,chain FROM audit_events ORDER BY sequence", &[])
        .map_err(port_error)?.into_iter().map(decode_event).collect()
}

fn decode_event(row: Row) -> Result<ChainedEvent, ApplicationError> {
    let sequence: i64 = row.get("sequence");
    let timestamp: String = row.get("timestamp");
    let timestamp = OffsetDateTime::parse(&timestamp, &Rfc3339)
        .map_err(|error| ApplicationError::Port(format!("invalid audit timestamp: {error}")))?;
    Ok(ChainedEvent {
        event: AuditEvent::new(
            u64::try_from(sequence)
                .map_err(|_| ApplicationError::Port("negative audit sequence".into()))?,
            timestamp,
            row.get::<_, String>("actor"),
            row.get::<_, String>("action"),
            row.get::<_, String>("resource"),
        ),
        chain: digest(row.get("chain"))?,
    })
}

fn digest(bytes: Vec<u8>) -> Result<Sha256Digest, ApplicationError> {
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| ApplicationError::Port("invalid audit digest length".into()))?;
    Ok(Sha256Digest::from_array(bytes))
}

impl AuditLog for PostgresAuditLog {
    fn append(
        &mut self,
        actor: &str,
        action: &str,
        resource: &str,
        timestamp: OffsetDateTime,
    ) -> Result<ChainedEvent, DomainError> {
        let mut client = self
            .client
            .lock()
            .map_err(|_| DomainError::AuditStorageFailure("audit database lock poisoned".into()))?;
        let mut transaction = begin_audited(&mut client).map_err(storage_error)?;
        let entry = append_transaction(&mut transaction, actor, action, resource, timestamp)
            .map_err(storage_error)?;
        transaction
            .commit()
            .map_err(|error| storage_error(port_error(error)))?;
        Ok(entry)
    }

    fn load_all(&self) -> Result<Vec<ChainedEvent>, DomainError> {
        let mut client = self
            .client
            .lock()
            .map_err(|_| DomainError::AuditStorageFailure("audit database lock poisoned".into()))?;
        load_transaction(&mut *client).map_err(storage_error)
    }
}

fn port_error(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("audit database: {error}"))
}

fn storage_error(error: ApplicationError) -> DomainError {
    DomainError::AuditStorageFailure(error.to_string())
}
