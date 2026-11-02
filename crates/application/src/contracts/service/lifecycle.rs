//! Creating and abandoning a contract. Neither has a request, so both are
//! reconciled through the receipt of the case route.

use domain::document_signing::Uuid;
use domain::{cases::CaseId, crypto::DocumentId, identity::Permission};

use super::{bounded_text, expect_revision, identity, ContractService, Error};
use crate::{
    contracts::{
        access::Relation,
        model::{ContractReceipt, ContractSnapshot, EventKind},
        notices,
        port::{ContractCommand, ContractWrite, ExpiryStep},
        state::{open_request, unclosed_request},
    },
    document_signing::SigningWorkflowError,
    ApplicationError,
};

pub(super) const TITLE_LIMIT: usize = 160;
pub(super) const REASON_LIMIT: usize = 500;

impl ContractService {
    /// Creates the contract of one document series; the server chooses its
    /// identifier, so an uncertain answer is reconciled by `operation`.
    pub fn create(
        &self,
        token: &str,
        case: CaseId,
        operation: Uuid,
        document: DocumentId,
        title: &str,
    ) -> Result<ContractReceipt, ApplicationError> {
        let principal = self.principal(token)?;
        identity(operation)?;
        identity(document.as_uuid())?;
        let title = bounded_text(title, TITLE_LIMIT)?;
        if !principal.role.allows(Permission::ManageContract) {
            return Err(Error::NotFound.into());
        }
        if let Some(receipt) = self.find_receipt(&principal, operation)? {
            if receipt.event.kind != EventKind::Created || receipt.case != case {
                return Err(SigningWorkflowError::Conflict.into());
            }
            self.reauthenticate(token, &principal)?;
            return Ok(receipt);
        }
        let at = self.now();
        let receipt = self
            .ports
            .store
            .commit(ContractCommand {
                actor: Some(principal.clone()),
                operation,
                revision: 1,
                at,
                write: ContractWrite::Create {
                    contract: Uuid::new_v4(),
                    case,
                    document,
                    title,
                },
                notices: Vec::new(),
            })?
            .into_receipt();
        if receipt.event.kind != EventKind::Created || receipt.case != case {
            return Err(Error::Inconsistent.into());
        }
        self.reauthenticate(token, &principal)?;
        Ok(receipt)
    }

    /// Abandons a contract in `borrador`, with a mandatory reason.
    #[allow(clippy::too_many_arguments)]
    pub fn abandon(
        &self,
        token: &str,
        case: CaseId,
        contract: Uuid,
        operation: Uuid,
        expected_revision: u32,
        reason: &str,
    ) -> Result<ContractReceipt, ApplicationError> {
        let principal = self.principal(token)?;
        identity(operation)?;
        let reason = bounded_text(reason, REASON_LIMIT)?;
        let snapshot = self
            .ports
            .store
            .contract(&principal, case, contract)?
            .ok_or(Error::NotFound)?;
        Relation::of(&principal, &snapshot, None).manage()?;
        if let Some(receipt) = self.find_receipt(&principal, operation)? {
            if receipt.event.kind != EventKind::Abandoned || receipt.contract != contract {
                return Err(SigningWorkflowError::Conflict.into());
            }
            self.reauthenticate(token, &principal)?;
            return Ok(receipt);
        }
        let at = self.now();
        let mut revision = expect_revision(&snapshot, expected_revision)?;
        require_draft(&snapshot, at.unix_timestamp())?;
        let expire_first = self.expiry_step(&snapshot, revision, at.unix_timestamp());
        if expire_first.is_some() {
            revision += 1;
        }
        let receipt = self
            .ports
            .store
            .commit(ContractCommand {
                actor: Some(principal.clone()),
                operation,
                revision,
                at,
                write: ContractWrite::Abandon {
                    contract,
                    reason,
                    expire_first,
                },
                notices: Vec::new(),
            })?
            .into_receipt();
        self.reauthenticate(token, &principal)?;
        Ok(receipt)
    }

    /// The `expired` event of a request that is exhausted but not closed,
    /// at `revision`, with its notices.
    pub(super) fn expiry_step(
        &self,
        snapshot: &ContractSnapshot,
        revision: u32,
        now: i64,
    ) -> Option<ExpiryStep> {
        let request = unclosed_request(snapshot).filter(|r| now >= r.expires_at())?;
        Some(ExpiryStep {
            request: request.id(),
            revision,
            operation: Uuid::new_v4(),
            notices: notices::notices(
                &self.ports.notices,
                EventKind::Expired,
                request.request.fields(),
                &snapshot.root.title,
            ),
        })
    }
}

/// A new request or an abandonment needs a contract in `borrador`.
pub(super) fn require_draft(snapshot: &ContractSnapshot, now: i64) -> Result<(), ApplicationError> {
    match snapshot.events.last().map(|event| event.kind) {
        Some(EventKind::Completed | EventKind::Abandoned) => Err(Error::RequestClosed.into()),
        None => Err(Error::Inconsistent.into()),
        _ if open_request(snapshot, now).is_some() => Err(Error::RequestOpen.into()),
        _ => Ok(()),
    }
}
