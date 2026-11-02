//! Stored rows to contract values; every stored statement is decoded with
//! the domain codecs, so a row that does not decode is inconsistent.

use super::errors::{inconsistent, port};
use application::{
    contracts::{
        Completion, ContractEvent, ContractRoot, EventKind, PartySignature, StatementToken,
    },
    ApplicationError,
};
use domain::{
    cases::CaseId, clock::OffsetDateTime, crypto::DocumentId,
    document_signing::contract::ContractPartyStatement, identity::UserId,
};
use postgres::Row;
use uuid::Uuid;

pub(super) fn get<'a, T: postgres::types::FromSql<'a>>(
    row: &'a Row,
    column: &str,
) -> Result<T, ApplicationError> {
    row.try_get(column).map_err(port)
}

pub(super) fn instant(
    row: &Row,
    seconds: &str,
    nanoseconds: &str,
) -> Result<OffsetDateTime, ApplicationError> {
    let seconds: i64 = get(row, seconds)?;
    let nanoseconds: i32 = get(row, nanoseconds)?;
    let value = i128::from(seconds) * 1_000_000_000 + i128::from(nanoseconds);
    OffsetDateTime::from_unix_timestamp_nanos(value).map_err(|_| inconsistent())
}

pub(super) fn root(row: &Row) -> Result<ContractRoot, ApplicationError> {
    Ok(ContractRoot {
        id: get(row, "id")?,
        case: CaseId::from_uuid(get(row, "case_id")?),
        document: DocumentId::from_uuid(get(row, "document_id")?),
        title: get(row, "title")?,
        created_by: UserId::from_uuid(get(row, "created_by")?),
        created_by_email: get(row, "created_by_email")?,
        created_at: instant(row, "created_at_seconds", "created_at_nanoseconds")?,
    })
}

pub(super) fn event(row: &Row) -> Result<ContractEvent, ApplicationError> {
    let kind: String = get(row, "kind")?;
    let revision: i64 = get(row, "revision")?;
    Ok(ContractEvent {
        revision: u32::try_from(revision).map_err(|_| inconsistent())?,
        kind: EventKind::parse(&kind).ok_or_else(inconsistent)?,
        request: get(row, "request_id")?,
        actor: get::<Option<Uuid>>(row, "actor_id")?.map(UserId::from_uuid),
        actor_label: get(row, "actor_label")?,
        reason: get(row, "reason")?,
        at: instant(row, "at_seconds", "at_nanoseconds")?,
        operation: get(row, "operation_id")?,
    })
}

/// A signature row joined with `root_der` and `crl_der` of its trust
/// revision.
pub(super) fn signature(row: &Row) -> Result<PartySignature, ApplicationError> {
    let bytes: Vec<u8> = get(row, "statement")?;
    let statement =
        ContractPartyStatement::from_canonical_bytes(&bytes).map_err(|_| inconsistent())?;
    let position: i16 = get(row, "position")?;
    if i16::from(statement.position().number()) != position {
        return Err(inconsistent());
    }
    let token: Option<Vec<u8>> = get(row, "timestamp_token")?;
    let chain: Option<Vec<u8>> = get(row, "tsa_chain_pem")?;
    let timestamp = match (token, chain) {
        (Some(token), Some(tsa_chain_pem)) => Some(StatementToken {
            token,
            tsa_chain_pem,
        }),
        (None, None) => None,
        _ => return Err(inconsistent()),
    };
    Ok(PartySignature {
        statement,
        signature: get(row, "signature")?,
        checked_at: get(row, "checked_at")?,
        valid_from: get(row, "valid_from")?,
        valid_until: get(row, "valid_until")?,
        accepted_at: instant(row, "accepted_at_seconds", "accepted_at_nanoseconds")?,
        timestamp,
        root_der: get(row, "root_der")?,
        crl_der: get(row, "crl_der")?,
    })
}

pub(super) fn completion(row: &Row) -> Result<Completion, ApplicationError> {
    Ok(Completion {
        document_token: get(row, "document_token")?,
        manifest: get(row, "manifest")?,
        manifest_token: get(row, "manifest_token")?,
        tsa_chain_pem: get(row, "tsa_chain_pem")?,
        completed_at: instant(row, "completed_at_seconds", "completed_at_nanoseconds")?,
    })
}
