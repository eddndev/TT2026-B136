//! Bilateral contract signing: contracts bound to one document series,
//! signature requests between a Client and a firm signer, their statements,
//! time-stamps, receipts and evidence packages, and the expiry closer.
//! Rationale: `docs/adr/0086-bilateral-contract-signing.md`.

mod access;
mod delivery;
mod error;
mod expiry;
mod model;
mod notices;
mod package;
mod port;
mod service;
mod state;
#[cfg(test)]
mod test_support;

pub use access::Relation;
pub use delivery::{
    ContractNoticeClaim, ContractNoticeFinish, ContractNoticeLease, ContractNoticeRun,
    ContractNoticeState, ContractNoticeStore, ContractNoticeWorker, NOTICE_BATCH,
    NOTICE_LEASE_SECONDS, NOTICE_PROVIDER_WINDOW_SECONDS,
};
pub use error::{ContractBlockers, ContractWorkflowError};
pub use expiry::{ContractExpiryService, EXPIRY_BATCH};
pub use model::{
    Completion, ContractCommit, ContractEvent, ContractReceipt, ContractRoot, ContractSnapshot,
    EventKind, PartySignature, RequestRecord, StatementToken, EXPIRY_ACTOR,
};
pub use notices::{recipients, NoticeSettings};
pub use port::{
    CaseContractsListing, ContractCommand, ContractNotice, ContractRead, ContractSignatureVerifier,
    ContractStore, ContractWrite, ExpiryStep, FirmSigner, RequestCandidates,
};
pub use service::{
    CaseContracts, CloseOutcome, ContractContent, ContractPackage, ContractPorts, ContractService,
    ContractStage, ContractView, NewRequest, PreparedContractSigning, RequestView,
    SubmittedSignature,
};
pub use state::{contract_state, open_request, request_status, ContractState, RequestStatus};
