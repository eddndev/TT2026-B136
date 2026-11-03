//! Closing an open request without completing it: the Client rejects
//! before signing, the firm signer declines after the Client signed, and an
//! Owner or the requester cancels with a reason. These are application
//! records that no package refutes.

use domain::document_signing::Uuid;

use super::{
    bounded_text, expect_revision, identity, lifecycle::REASON_LIMIT, ContractService, Error,
};
use crate::{
    contracts::{
        access::Relation,
        model::{ContractReceipt, EventKind},
        notices,
        port::{ContractCommand, ContractWrite},
        state::{request_status, RequestStatus},
    },
    document_signing::SigningWorkflowError,
    ApplicationError,
};

/// How a party closes a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseOutcome {
    Reject,
    Decline,
    Cancel,
}

impl CloseOutcome {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "reject" => Some(Self::Reject),
            "decline" => Some(Self::Decline),
            "cancel" => Some(Self::Cancel),
            _ => None,
        }
    }

    pub const fn kind(self) -> EventKind {
        match self {
            Self::Reject => EventKind::Rejected,
            Self::Decline => EventKind::Declined,
            Self::Cancel => EventKind::Cancelled,
        }
    }
}

impl ContractService {
    pub fn close(
        &self,
        token: &str,
        request: Uuid,
        operation: Uuid,
        expected_revision: u32,
        outcome: CloseOutcome,
        reason: Option<&str>,
    ) -> Result<ContractReceipt, ApplicationError> {
        let principal = self.principal(token)?;
        identity(operation)?;
        let kind = outcome.kind();
        let reason = match reason {
            Some(value) => Some(bounded_text(value, REASON_LIMIT)?),
            None if kind == EventKind::Cancelled => return Err(Error::Input.into()),
            None => None,
        };
        let snapshot = self.by_request(&principal, request)?;
        let record = snapshot.request(request).ok_or(Error::Inconsistent)?;
        Relation::of(&principal, &snapshot, Some(record)).close(kind)?;
        if let Some(receipt) = self.find_receipt(&principal, operation)? {
            if receipt.event.kind != kind
                || receipt.event.request != Some(request)
                || receipt.event.reason != reason
            {
                return Err(SigningWorkflowError::Conflict.into());
            }
            self.reauthenticate(token, &principal)?;
            return Ok(receipt);
        }
        let at = self.now();
        let revision = expect_revision(&snapshot, expected_revision)?;
        let status = request_status(&snapshot, record, at.unix_timestamp());
        match (kind, status) {
            (EventKind::Rejected, RequestStatus::AwaitingClient)
            | (EventKind::Declined, RequestStatus::AwaitingFirm)
            | (EventKind::Cancelled, RequestStatus::AwaitingClient | RequestStatus::AwaitingFirm) =>
                {}
            (_, RequestStatus::AwaitingClient | RequestStatus::AwaitingFirm) => {
                return Err(Error::OutOfTurn.into())
            }
            (_, RequestStatus::Expired) if !closed(&snapshot, request) => {
                return Err(Error::Expired.into())
            }
            _ => return Err(Error::RequestClosed.into()),
        }
        let notices = notices::notices(
            &self.ports.notices,
            kind,
            record.request.fields(),
            &snapshot.root.title,
        );
        let receipt = self
            .ports
            .store
            .commit(ContractCommand {
                actor: Some(principal.clone()),
                operation,
                revision,
                at,
                write: ContractWrite::Close {
                    contract: snapshot.root.id,
                    request,
                    kind,
                    reason,
                },
                notices,
            })?
            .into_receipt();
        if receipt.event.kind != kind || receipt.event.request != Some(request) {
            return Err(Error::Inconsistent.into());
        }
        self.reauthenticate(token, &principal)?;
        Ok(receipt)
    }
}

fn closed(snapshot: &crate::contracts::model::ContractSnapshot, request: Uuid) -> bool {
    snapshot
        .request_events(request)
        .any(|event| event.kind.closes_unsuccessfully())
}
