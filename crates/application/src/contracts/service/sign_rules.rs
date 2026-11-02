//! The pure rules of a signature submission: whose turn it is, the token
//! bounds, the comparison of a submitted statement with the expected one,
//! and the exact replay of an operation.

use domain::document_signing::{contract::ContractPartyStatement, Uuid};

use super::{sign::SubmittedSignature, Error};
use crate::{
    contracts::{
        model::{ContractReceipt, ContractSnapshot, RequestRecord},
        state::{request_status, RequestStatus},
    },
    document_signing::{SigningWorkflowError as Signing, TokenFacts},
    ApplicationError,
};

/// Whose turn it is: the request must be open, at the caller's position,
/// on the latest version of the series, in an open case.
pub(super) fn turn(
    snapshot: &ContractSnapshot,
    record: &RequestRecord,
    position: u8,
    now: i64,
) -> Result<(), ApplicationError> {
    let status = request_status(snapshot, record, now);
    match (status, position) {
        (RequestStatus::AwaitingClient, 1) | (RequestStatus::AwaitingFirm, 2) => {}
        (RequestStatus::AwaitingClient | RequestStatus::AwaitingFirm, _) => {
            return Err(Error::OutOfTurn.into())
        }
        (RequestStatus::Expired, _) if !closed_by_event(snapshot, record) => {
            return Err(Error::Expired.into())
        }
        _ => return Err(Error::RequestClosed.into()),
    }
    if snapshot.latest_version > record.request.fields().document.version.get() {
        return Err(Error::VersionSuperseded.into());
    }
    if snapshot.case_closed {
        return Err(Error::RequestClosed.into());
    }
    Ok(())
}

pub(super) fn closed_by_event(snapshot: &ContractSnapshot, record: &RequestRecord) -> bool {
    snapshot
        .request_events(record.id())
        .any(|event| event.kind.closes_unsuccessfully())
}

/// The token must follow the request creation and, with its accuracy,
/// precede the expiry.
pub(super) fn within(
    facts: &TokenFacts,
    created: i64,
    expires: i64,
) -> Result<(), ApplicationError> {
    if facts.gen_time < created {
        return Err(Error::TimestampUnavailable.into());
    }
    if facts.gen_time.saturating_add(facts.accuracy_seconds) > expires {
        return Err(Error::Expired.into());
    }
    Ok(())
}

/// The token, less its accuracy, must not follow the instant of acceptance;
/// an authority whose clock runs ahead wrote nothing and may be retried.
pub(super) fn token_before(facts: &TokenFacts, accepted: i64) -> Result<(), ApplicationError> {
    crate::document_signing::check_before(facts, accepted)
        .map_err(|_| Error::TimestampUnavailable.into())
}

/// The submitted statement against the one built from current facts.
pub(super) fn compare(
    submitted: &ContractPartyStatement,
    expected: &ContractPartyStatement,
) -> Result<(), ApplicationError> {
    if submitted.account() != expected.account() {
        return Err(Signing::AccountChanged.into());
    }
    let (a, b) = (submitted.material(), expected.material());
    if a.deployment() != b.deployment()
        || a.root() != b.root()
        || a.trust_revision() != b.trust_revision()
    {
        return Err(Signing::TrustChanged.into());
    }
    if submitted != expected {
        return Err(Signing::Conflict.into());
    }
    Ok(())
}

/// The receipt must hold exactly the submitted statement and signature.
pub(super) fn same_signature(
    receipt: &ContractReceipt,
    request: Uuid,
    submitted: &SubmittedSignature,
) -> Result<(), ApplicationError> {
    let matches = receipt.event.request == Some(request)
        && receipt.signature.as_ref().is_some_and(|signature| {
            signature.statement.canonical_bytes()[..] == submitted.statement[..]
                && signature.signature == submitted.signature
        });
    if !matches {
        return Err(Signing::Conflict.into());
    }
    Ok(())
}
