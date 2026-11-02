//! Outcomes of the contract workflow, one per error code of
//! `docs/adr/0086-bilateral-contract-signing.md`. They never carry submitted
//! identifiers, certificates or signatures.

use serde::Serialize;

/// Which preconditions of a new request are not met. `client_not_ready`
/// never distinguishes a missing, ineligible or unbound account.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ContractBlockers {
    pub client_not_ready: bool,
    pub firm_signer_not_ready: bool,
    pub document_not_ready: bool,
}

impl ContractBlockers {
    pub const fn any(&self) -> bool {
        self.client_not_ready || self.firm_signer_not_ready || self.document_not_ready
    }

    /// The stable codes of the set blockers, in a fixed order.
    pub fn codes(&self) -> Vec<&'static str> {
        [
            (self.client_not_ready, "client_not_ready"),
            (self.firm_signer_not_ready, "firm_signer_not_ready"),
            (self.document_not_ready, "document_not_ready"),
        ]
        .into_iter()
        .filter_map(|(set, code)| set.then_some(code))
        .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ContractWorkflowError {
    #[error("contract was not found")]
    NotFound,
    #[error("the document series already has a contract")]
    Exists,
    #[error("contract revision differs from the expected revision")]
    RevisionConflict,
    #[error("an open contract request already exists")]
    RequestOpen,
    #[error("contract request is closed")]
    RequestClosed,
    #[error("contract signature is out of turn")]
    OutOfTurn,
    #[error("contract request validity is exhausted")]
    Expired,
    #[error("a later version of the contracted document exists")]
    VersionSuperseded,
    #[error("contracted document version is frozen")]
    VersionFrozen,
    #[error("client certificate is revoked in the current revocation list")]
    CounterpartyRevoked,
    #[error("contract is not complete")]
    NotComplete,
    #[error("contract party is not eligible")]
    PartyIneligible(ContractBlockers),
    #[error("time-stamping authority is unavailable; no state was written")]
    TimestampUnavailable,
    #[error("contract input is invalid")]
    Input,
    #[error("contract evidence is inconsistent")]
    Inconsistent,
}
