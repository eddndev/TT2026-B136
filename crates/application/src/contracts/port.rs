//! Ports of the contract workflow. Stores read with the caller's identity
//! and return `None` when the caller has neither staff scope on the case nor
//! a designation in one of its requests; every write rechecks authority,
//! revision and eligibility under the contract locks and commits its event,
//! audit and notifications in one transaction. No method retries.

use domain::document_signing::Uuid;
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::{CredentialCheck, CredentialFailure, DocumentId, Signature},
    document_signing::contract::{ContractPartyStatement, ContractRequest},
    identity::UserId,
};

use super::model::{
    Completion, ContractCommit, ContractReceipt, ContractSnapshot, EventKind, StatementToken,
};
use crate::{
    credential_trust::CredentialTrustSnapshot,
    document_signing::{SigningBindingReceipt, SigningWorkflowError},
    documents::DocumentRecord,
    identity::Principal,
    ApplicationError,
};

/// Verifies a contract party statement under the internal declaration
/// profile with mandatory CRL checking.
pub trait ContractSignatureVerifier: Send + Sync {
    /// Verifies `signature` over the SHA-256 of the 303 statement bytes and
    /// that the statement names the root, revision and leaf of the check.
    fn verify_statement(
        &self,
        statement: &ContractPartyStatement,
        certificate: &[u8],
        signature: &Signature,
        trust: &CredentialTrustSnapshot,
        at: i64,
    ) -> Result<CredentialCheck, SigningWorkflowError>;

    /// The status of `certificate` under `trust` at `at`.
    fn certificate_status(
        &self,
        certificate: &[u8],
        trust: &CredentialTrustSnapshot,
        at: i64,
    ) -> Result<(), CredentialFailure>;
}

/// One notification row; the store resolves the recipient's address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractNotice {
    pub recipient: UserId,
    pub from: String,
    pub subject: String,
    pub text: String,
}

/// The parties and document a new request would name, as the store finds
/// them for the caller. A party is `None` when it is missing, ineligible or
/// unbound, without distinction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestCandidates {
    pub client: Option<(UserId, SigningBindingReceipt)>,
    pub firm_signer: Option<(UserId, domain::identity::Role, SigningBindingReceipt)>,
    pub document: Option<DocumentRecord>,
    pub trust: Option<CredentialTrustSnapshot>,
}

/// One append-only mutation; `revision` is the revision of its event. A
/// command is built once per request and moved to the store, so the size
/// of its largest variant costs nothing worth boxing.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum ContractWrite {
    Create {
        contract: Uuid,
        case: CaseId,
        document: DocumentId,
        title: String,
    },
    Abandon {
        contract: Uuid,
        reason: String,
        expire_first: Option<ExpiryStep>,
    },
    Request {
        contract: Uuid,
        request: ContractRequest,
        client_binding: Uuid,
        firm_binding: Uuid,
        ceremony_reference: String,
        subject: Option<(Uuid, u64)>,
        /// The `expired` event of an exhausted previous request, written
        /// first in the same transaction.
        expire_first: Option<ExpiryStep>,
    },
    Sign {
        contract: Uuid,
        request: Uuid,
        statement: ContractPartyStatement,
        signature: Vec<u8>,
        check: CredentialCheck,
        timestamp: Option<StatementToken>,
        completion: Option<Completion>,
    },
    Close {
        contract: Uuid,
        request: Uuid,
        kind: EventKind,
        reason: Option<String>,
    },
}

/// The audited `expired` event of a request, with its notices.
#[derive(Debug, Clone)]
pub struct ExpiryStep {
    pub request: Uuid,
    pub revision: u32,
    pub operation: Uuid,
    pub notices: Vec<ContractNotice>,
}

/// A mutation with its author, operation, event revision, instant and
/// notices. `actor` is `None` only for the expiry closer.
#[derive(Debug, Clone)]
pub struct ContractCommand {
    pub actor: Option<Principal>,
    pub operation: Uuid,
    pub revision: u32,
    pub at: OffsetDateTime,
    pub write: ContractWrite,
    pub notices: Vec<ContractNotice>,
}

/// What a read of exact content or evidence records in the audit chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractRead {
    Content,
    PartialEvidence,
    FinalEvidence,
}

/// An account that may sign for the firm in a case: an active Owner, or an
/// active Litigator member, with a live binding under the current root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmSigner {
    pub id: UserId,
    pub email: String,
    pub role: domain::identity::Role,
}

/// The contracts of a case, at most 50, newest first, and its eligible
/// firm signers, at most 50.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseContractsListing {
    pub contracts: Vec<ContractSnapshot>,
    pub firm_signers: Vec<FirmSigner>,
}

pub trait ContractStore: Send + Sync {
    /// Requires staff scope on the case and `ReadContract`; anyone else
    /// receives `contract_not_found`.
    fn case_contracts(
        &self,
        principal: &Principal,
        case: CaseId,
    ) -> Result<CaseContractsListing, ApplicationError>;
    fn contract(
        &self,
        principal: &Principal,
        case: CaseId,
        contract: Uuid,
    ) -> Result<Option<ContractSnapshot>, ApplicationError>;
    fn contract_by_request(
        &self,
        principal: &Principal,
        request: Uuid,
    ) -> Result<Option<ContractSnapshot>, ApplicationError>;
    /// Contracts with a request designating the caller as Client or firm
    /// signer, at most 50, newest request first.
    fn party_contracts(
        &self,
        principal: &Principal,
    ) -> Result<Vec<ContractSnapshot>, ApplicationError>;
    /// The receipt of a contract mutation, found by its operation, when the
    /// caller has staff scope on its case or is a party of its request.
    fn receipt(
        &self,
        principal: &Principal,
        operation: Uuid,
    ) -> Result<Option<ContractReceipt>, ApplicationError>;
    /// Resolves the exact `client_email`, the firm signer and the latest
    /// version of the contracted series for a new request.
    fn request_candidates(
        &self,
        principal: &Principal,
        contract: Uuid,
        client_email: &str,
        firm_signer: UserId,
    ) -> Result<RequestCandidates, ApplicationError>;
    /// The exact stored version a request names, with its encrypted vault.
    fn request_document(
        &self,
        principal: &Principal,
        request: Uuid,
    ) -> Result<DocumentRecord, ApplicationError>;
    /// Commits one mutation; the exact replay of an operation returns
    /// `Existing` with the original receipt and a different one conflicts.
    fn commit(&self, command: ContractCommand) -> Result<ContractCommit, ApplicationError>;
    /// Appends the audit of a content read or an evidence export.
    fn record_read(
        &self,
        principal: &Principal,
        request: Uuid,
        read: ContractRead,
        at: OffsetDateTime,
    ) -> Result<(), ApplicationError>;
    /// Requests whose validity is exhausted and that have no closing event,
    /// at most `limit`, oldest expiry first.
    fn exhausted(&self, at: i64, limit: usize) -> Result<Vec<ContractSnapshot>, ApplicationError>;
}
