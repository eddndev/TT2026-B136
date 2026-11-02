//! Who may do what with a contract: staff by role and case scope, parties
//! by their designation in one request. Every refusal of a caller without
//! any relation reads as `contract_not_found`. Rationale: the section
//! "Access" of `docs/adr/0086-bilateral-contract-signing.md`.

use domain::identity::{Permission, Role};

use super::{
    error::ContractWorkflowError as Error,
    model::{ContractSnapshot, RequestRecord},
    state::RequestStatus,
};
use crate::{identity::Principal, ApplicationError};

/// The caller's relation to one contract and, optionally, one request.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Relation {
    pub staff_read: bool,
    pub staff_manage: bool,
    pub staff_export: bool,
    pub owner: bool,
    pub client: bool,
    pub firm: bool,
    /// The designated firm signer who still has staff scope on the case.
    /// Every firm signer is an Owner or a Litigator; one who lost the case
    /// keeps only the request view, like any other former staff.
    pub firm_scoped: bool,
    pub requester: bool,
}

impl Relation {
    pub fn of(
        principal: &Principal,
        snapshot: &ContractSnapshot,
        request: Option<&RequestRecord>,
    ) -> Self {
        let scoped = |permission| snapshot.case_scope && principal.role.allows(permission);
        let fields = request.map(|request| *request.request.fields());
        Self {
            staff_read: scoped(Permission::ReadContract),
            staff_manage: scoped(Permission::ManageContract),
            staff_export: scoped(Permission::ReadContract) && scoped(Permission::ExportEvidence),
            owner: snapshot.case_scope && principal.role == Role::Owner,
            client: fields.is_some_and(|f| f.client == principal.id),
            firm: fields.is_some_and(|f| f.firm_signer == principal.id),
            firm_scoped: snapshot.case_scope
                && fields.is_some_and(|f| f.firm_signer == principal.id),
            requester: fields.is_some_and(|f| f.requester == principal.id),
        }
    }

    fn party(&self) -> bool {
        self.client || self.firm
    }

    /// Reading a contract through its case.
    pub fn read_contract(&self) -> Result<(), ApplicationError> {
        allow(self.staff_read)
    }

    /// Creating, abandoning and requesting signatures.
    pub fn manage(&self) -> Result<(), ApplicationError> {
        if self.staff_manage {
            return Ok(());
        }
        self.denied()
    }

    /// Reading one request, its state and events.
    pub fn read_request(&self) -> Result<(), ApplicationError> {
        allow(self.staff_read || self.party())
    }

    /// The exact PDF through the request route: the firm signer with case
    /// scope while the request is open, the Client while it is open or once
    /// they signed.
    pub fn read_content(
        &self,
        status: RequestStatus,
        client_signed: bool,
    ) -> Result<(), ApplicationError> {
        allow(
            (self.firm_scoped && status.is_open())
                || (self.client && (status.is_open() || client_signed)),
        )
    }

    /// The partial package: staff with export scope, which includes a firm
    /// signer who keeps the case, and the Client once signed, also after an
    /// unsuccessful close.
    pub fn partial_package(&self, client_signed: bool) -> Result<(), ApplicationError> {
        allow(self.staff_export || (self.client && client_signed))
    }

    /// The full package: staff with export scope and the Client.
    pub fn final_package(&self) -> Result<(), ApplicationError> {
        allow(self.staff_export || self.client)
    }

    /// Signing position 1 or 2; staff who can read but are not designated,
    /// and a firm signer who lost the case, are refused without the
    /// not-found answer.
    pub fn sign(&self, position: u8) -> Result<(), ApplicationError> {
        let designated = match position {
            1 => self.client,
            2 => self.firm_scoped,
            _ => false,
        };
        if designated {
            return Ok(());
        }
        self.denied()
    }

    /// Rejecting (Client), declining (firm signer) or cancelling (an Owner,
    /// or the requester who still manages the case).
    pub fn close(&self, kind: super::model::EventKind) -> Result<(), ApplicationError> {
        use super::model::EventKind;
        let allowed = match kind {
            EventKind::Rejected => self.client,
            EventKind::Declined => self.firm_scoped,
            EventKind::Cancelled => self.owner || (self.requester && self.staff_manage),
            _ => false,
        };
        if allowed {
            return Ok(());
        }
        self.denied()
    }

    /// A caller who can see the contract learns that the action is not
    /// theirs; anyone else learns nothing.
    fn denied(&self) -> Result<(), ApplicationError> {
        if self.staff_read || self.party() {
            return Err(ApplicationError::PermissionDenied);
        }
        Err(Error::NotFound.into())
    }
}

fn allow(allowed: bool) -> Result<(), ApplicationError> {
    if allowed {
        return Ok(());
    }
    Err(Error::NotFound.into())
}

#[cfg(test)]
#[path = "access_tests.rs"]
mod tests;
