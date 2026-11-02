//! What document reads say about a contracted series, and the freeze that
//! seals and individual signatures check before writing evidence. The
//! single definition of an open request applies: its last event is
//! `requested` or `client_signed` and its validity is not exhausted.
//! Rationale: docs/adr/0086-bilateral-contract-signing.md.

use application::{
    contracts::{ContractState, ContractWorkflowError},
    ApplicationError,
};
use domain::crypto::{DocumentId, DocumentVersion};
use postgres::{Row, Transaction};

use super::storage::port_error;

/// The derived state of the contract of series `document`, at Unix
/// seconds `now`, or NULL without a contract.
pub(super) fn state_column(document: &str, now: &str) -> String {
    format!(
        "(SELECT CASE WHEN e.kind='completed' THEN 'firmado' WHEN e.kind='abandoned' THEN 'abandonado'
            WHEN e.kind IN ('requested','client_signed') AND r.expires_at_seconds>{now} THEN 'en_firma'
            ELSE 'borrador' END
        FROM contracts c CROSS JOIN LATERAL (SELECT kind,request_id FROM contract_events
            WHERE contract_id=c.id ORDER BY revision DESC LIMIT 1) e
        LEFT JOIN contract_signature_requests r ON r.request_id=e.request_id
        WHERE c.document_id={document})"
    )
}

/// Whether version `version` of `document` is named by a completed request
/// or by one that no event closed and whose validity runs past `now`.
pub(super) fn frozen_predicate(document: &str, version: &str, now: &str) -> String {
    format!(
        "EXISTS(SELECT 1 FROM contract_signature_requests r
            WHERE r.document_id={document} AND r.document_version={version}
            AND (EXISTS(SELECT 1 FROM contract_events e WHERE e.request_id=r.request_id
                    AND e.kind='completed')
                OR (r.expires_at_seconds>{now} AND NOT EXISTS(SELECT 1 FROM contract_events e
                    WHERE e.request_id=r.request_id
                    AND e.kind IN ('rejected','declined','cancelled','expired')))))"
    )
}

pub(super) fn state(row: &Row) -> Result<Option<ContractState>, ApplicationError> {
    let value: Option<String> = row.try_get("contract_state").map_err(port_error)?;
    value
        .map(|value| {
            ContractState::parse(&value)
                .ok_or_else(|| ApplicationError::Port("invalid derived contract state".into()))
        })
        .transpose()
}

/// The derived state of the contract of series `document` at `now`.
pub(super) fn of(
    transaction: &mut Transaction<'_>,
    document: DocumentId,
    now: i64,
) -> Result<Option<ContractState>, ApplicationError> {
    let sql = format!(
        "SELECT {} AS contract_state",
        state_column("$1", "$2::bigint")
    );
    let row = transaction
        .query_one(&sql, &[&document.as_uuid(), &now])
        .map_err(port_error)?;
    state(&row)
}

/// Refuses evidence for a frozen version with the database clock, the same
/// instant the storage guard reads, so the answer is `contract_version_frozen`
/// rather than a trigger failure.
pub(crate) fn require_unfrozen(
    transaction: &mut Transaction<'_>,
    document: DocumentId,
    version: DocumentVersion,
) -> Result<(), ApplicationError> {
    let sql = format!(
        "SELECT {}",
        frozen_predicate("$1", "$2", "extract(epoch FROM clock_timestamp())")
    );
    let frozen: bool = transaction
        .query_one(&sql, &[&document.as_uuid(), &i64::from(version.get())])
        .map_err(port_error)?
        .get(0);
    if frozen {
        return Err(ContractWorkflowError::VersionFrozen.into());
    }
    Ok(())
}
