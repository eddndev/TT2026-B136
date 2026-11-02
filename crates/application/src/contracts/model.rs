//! Stored contract history as the store returns it: an immutable root, its
//! append-only events, its signature requests with their party signatures
//! and completion. No value here holds a derived state.

use domain::document_signing::Uuid;
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::{DocumentId, Sha256Digest},
    document_signing::contract::{ContractPartyStatement, ContractRequest},
    identity::UserId,
};

use crate::document_signing::SigningBindingReceipt;

/// The technical actor label of every `expired` event.
pub const EXPIRY_ACTOR: &str = "system:contract-expiry";

/// The kind of one contract event, written as its storage name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventKind {
    Created,
    Abandoned,
    Requested,
    ClientSigned,
    Rejected,
    Completed,
    Declined,
    Cancelled,
    Expired,
}

impl EventKind {
    pub const ALL: [EventKind; 9] = [
        EventKind::Created,
        EventKind::Abandoned,
        EventKind::Requested,
        EventKind::ClientSigned,
        EventKind::Rejected,
        EventKind::Completed,
        EventKind::Declined,
        EventKind::Cancelled,
        EventKind::Expired,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Abandoned => "abandoned",
            Self::Requested => "requested",
            Self::ClientSigned => "client_signed",
            Self::Rejected => "rejected",
            Self::Completed => "completed",
            Self::Declined => "declined",
            Self::Cancelled => "cancelled",
            Self::Expired => "expired",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }

    /// The audit action this event commits with.
    pub fn action(self) -> String {
        format!("contract.{}", self.as_str())
    }

    /// Closes a request without completing it.
    pub const fn closes_unsuccessfully(self) -> bool {
        matches!(
            self,
            Self::Rejected | Self::Declined | Self::Cancelled | Self::Expired
        )
    }
}

/// One append-only event; `actor` is `None` only for `expired`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractEvent {
    pub revision: u32,
    pub kind: EventKind,
    pub request: Option<Uuid>,
    pub actor: Option<UserId>,
    pub actor_label: String,
    pub reason: Option<String>,
    pub at: OffsetDateTime,
    pub operation: Uuid,
}

/// The immutable contract root bound to one document series.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractRoot {
    pub id: Uuid,
    pub case: CaseId,
    pub document: DocumentId,
    pub title: String,
    pub created_by: UserId,
    pub created_by_email: String,
    pub created_at: OffsetDateTime,
}

/// One party signature with the trust revision it was checked against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartySignature {
    pub statement: ContractPartyStatement,
    pub signature: Vec<u8>,
    pub checked_at: i64,
    pub valid_from: i64,
    pub valid_until: i64,
    pub accepted_at: OffsetDateTime,
    /// T1 and its anchor candidates, in position 1 only.
    pub timestamp: Option<StatementToken>,
    /// Root and CRL of the trust revision the statement names.
    pub root_der: Vec<u8>,
    pub crl_der: Vec<u8>,
}

impl PartySignature {
    pub fn position(&self) -> u8 {
        self.statement.position().number()
    }
}

/// A time-stamp response and the chain that anchors it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementToken {
    pub token: Vec<u8>,
    pub tsa_chain_pem: Vec<u8>,
}

/// Td, the manifest and Tf of a completed request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    pub document_token: Vec<u8>,
    pub manifest: Vec<u8>,
    pub manifest_token: Vec<u8>,
    pub tsa_chain_pem: Vec<u8>,
    pub completed_at: OffsetDateTime,
}

/// A signature request with both live-at-request bindings, which carry
/// each party's certificate, binding proof and registration trust.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestRecord {
    pub request: ContractRequest,
    pub requester_email: String,
    pub ceremony_reference: String,
    pub subject: Option<(Uuid, u64)>,
    pub created_at: OffsetDateTime,
    pub client_binding: SigningBindingReceipt,
    pub firm_binding: SigningBindingReceipt,
    pub client_role: domain::identity::Role,
    pub firm_role: domain::identity::Role,
    pub signatures: Vec<PartySignature>,
    pub completion: Option<Completion>,
}

impl RequestRecord {
    pub fn id(&self) -> Uuid {
        self.request.fields().request
    }

    pub fn bytes(&self) -> [u8; 240] {
        self.request.canonical_bytes()
    }

    pub fn expires_at(&self) -> i64 {
        self.request.fields().expires_at_seconds
    }

    pub fn signature(&self, position: u8) -> Option<&PartySignature> {
        self.signatures
            .iter()
            .find(|signature| signature.position() == position)
    }
}

/// Everything stored about one contract, plus the facts its readers need:
/// whether the caller has staff scope on the case, the latest version of
/// the series and whether the case is closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractSnapshot {
    pub root: ContractRoot,
    pub events: Vec<ContractEvent>,
    pub requests: Vec<RequestRecord>,
    pub case_scope: bool,
    pub latest_version: u32,
    pub case_closed: bool,
}

impl ContractSnapshot {
    pub fn revision(&self) -> u32 {
        self.events.last().map_or(0, |event| event.revision)
    }

    pub fn request(&self, id: Uuid) -> Option<&RequestRecord> {
        self.requests.iter().find(|request| request.id() == id)
    }

    /// The events of one request, in revision order.
    pub fn request_events(&self, id: Uuid) -> impl Iterator<Item = &ContractEvent> {
        self.events
            .iter()
            .filter(move |event| event.request == Some(id))
    }
}

/// The exact result of one committed mutation, found by its operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractReceipt {
    pub contract: Uuid,
    pub case: CaseId,
    pub event: ContractEvent,
    /// The 240 request bytes for a `requested` event.
    pub request_statement: Option<Vec<u8>>,
    /// The party statement and signature for a signing event.
    pub signature: Option<PartySignature>,
    /// SHA-256 of the manifest for a `completed` event.
    pub manifest_sha256: Option<Sha256Digest>,
}

/// Distinguishes a new commit from the recovery of the same operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractCommit {
    Applied(ContractReceipt),
    Existing(ContractReceipt),
}

impl ContractCommit {
    pub fn into_receipt(self) -> ContractReceipt {
        match self {
            Self::Applied(receipt) | Self::Existing(receipt) => receipt,
        }
    }
}
