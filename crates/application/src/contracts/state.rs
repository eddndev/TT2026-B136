//! Derived contract and request states. The single definition of an open
//! request: its last event is `requested` or `client_signed` and
//! `now < expires_at`. Rationale: the section "Contract entity and derived
//! state" of `docs/adr/0086-bilateral-contract-signing.md`.

use domain::document_signing::Uuid;
use serde::Serialize;

use super::model::{ContractSnapshot, EventKind, RequestRecord};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ContractState {
    Draft,
    Signing,
    Signed,
    Abandoned,
}

impl ContractState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "borrador",
            Self::Signing => "en_firma",
            Self::Signed => "firmado",
            Self::Abandoned => "abandonado",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        [Self::Draft, Self::Signing, Self::Signed, Self::Abandoned]
            .into_iter()
            .find(|state| state.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RequestStatus {
    AwaitingClient,
    AwaitingFirm,
    Completed,
    Rejected,
    Declined,
    Cancelled,
    /// Recorded as `expired`, or exhausted before that event exists.
    Expired,
}

impl RequestStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AwaitingClient => "awaiting_client",
            Self::AwaitingFirm => "awaiting_firm",
            Self::Completed => "completed",
            Self::Rejected => "rejected",
            Self::Declined => "declined",
            Self::Cancelled => "cancelled",
            Self::Expired => "expired",
        }
    }

    pub const fn is_open(self) -> bool {
        matches!(self, Self::AwaitingClient | Self::AwaitingFirm)
    }
}

/// The last recorded event kind of `request`, if any.
fn last_kind(snapshot: &ContractSnapshot, request: Uuid) -> Option<EventKind> {
    snapshot.request_events(request).last().map(|e| e.kind)
}

/// The status of one request at `now`, Unix seconds.
pub fn request_status(
    snapshot: &ContractSnapshot,
    request: &RequestRecord,
    now: i64,
) -> RequestStatus {
    let exhausted = now >= request.expires_at();
    match last_kind(snapshot, request.id()) {
        Some(EventKind::ClientSigned) if !exhausted => RequestStatus::AwaitingFirm,
        Some(EventKind::Requested) if !exhausted => RequestStatus::AwaitingClient,
        Some(EventKind::Completed) => RequestStatus::Completed,
        Some(EventKind::Rejected) => RequestStatus::Rejected,
        Some(EventKind::Declined) => RequestStatus::Declined,
        Some(EventKind::Cancelled) => RequestStatus::Cancelled,
        _ => RequestStatus::Expired,
    }
}

/// The request whose last event is `requested` or `client_signed`, open or
/// exhausted, which a new request or an abandonment must expire first.
pub fn unclosed_request(snapshot: &ContractSnapshot) -> Option<&RequestRecord> {
    let last = snapshot.events.last()?;
    if !matches!(last.kind, EventKind::Requested | EventKind::ClientSigned) {
        return None;
    }
    snapshot.request(last.request?)
}

/// The open request at `now`, if any.
pub fn open_request(snapshot: &ContractSnapshot, now: i64) -> Option<&RequestRecord> {
    unclosed_request(snapshot).filter(|request| now < request.expires_at())
}

/// The contract state at `now`, Unix seconds.
pub fn contract_state(snapshot: &ContractSnapshot, now: i64) -> ContractState {
    match snapshot.events.last().map(|event| event.kind) {
        Some(EventKind::Completed) => ContractState::Signed,
        Some(EventKind::Abandoned) => ContractState::Abandoned,
        _ if open_request(snapshot, now).is_some() => ContractState::Signing,
        _ => ContractState::Draft,
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
