//! Reads: the contracts of a case, one contract, the caller's own requests,
//! one request and the exact receipt of a mutation.

use domain::cases::CaseId;
use domain::document_signing::Uuid;

use super::{ContractService, Error};
use crate::{
    contracts::{
        access::Relation,
        model::{ContractReceipt, ContractSnapshot},
        port::FirmSigner,
        state::{contract_state, request_status, ContractState, RequestStatus},
    },
    ApplicationError,
};

/// One contract with its derived state and the status of each request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractView {
    pub snapshot: ContractSnapshot,
    pub state: ContractState,
    pub statuses: Vec<(Uuid, RequestStatus)>,
    /// The instant, Unix seconds, the states were derived at.
    pub derived_at: i64,
}

impl ContractView {
    pub fn new(snapshot: ContractSnapshot, now: i64) -> Self {
        let statuses = snapshot
            .requests
            .iter()
            .map(|request| (request.id(), request_status(&snapshot, request, now)))
            .collect();
        Self {
            state: contract_state(&snapshot, now),
            statuses,
            snapshot,
            derived_at: now,
        }
    }

    pub fn status(&self, request: Uuid) -> Option<RequestStatus> {
        self.statuses
            .iter()
            .find(|(id, _)| *id == request)
            .map(|(_, status)| *status)
    }
}

/// The contracts of a case and the accounts that may sign for the firm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseContracts {
    pub contracts: Vec<ContractView>,
    pub firm_signers: Vec<FirmSigner>,
}

/// One request as a party or a staff reader sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestView {
    pub contract: ContractView,
    pub request: Uuid,
    pub status: RequestStatus,
}

impl ContractService {
    pub fn case_contracts(
        &self,
        token: &str,
        case: CaseId,
    ) -> Result<CaseContracts, ApplicationError> {
        let principal = self.principal(token)?;
        if case.as_uuid().is_nil() {
            return Err(Error::NotFound.into());
        }
        let listing = self.ports.store.case_contracts(&principal, case)?;
        let now = self.now().unix_timestamp();
        let mut contracts = Vec::with_capacity(listing.contracts.len());
        for snapshot in listing.contracts {
            Relation::of(&principal, &snapshot, None).read_contract()?;
            if snapshot.root.case != case {
                return Err(Error::Inconsistent.into());
            }
            contracts.push(ContractView::new(snapshot, now));
        }
        self.reauthenticate(token, &principal)?;
        Ok(CaseContracts {
            contracts,
            firm_signers: listing.firm_signers,
        })
    }

    pub fn contract(
        &self,
        token: &str,
        case: CaseId,
        contract: Uuid,
    ) -> Result<ContractView, ApplicationError> {
        let principal = self.principal(token)?;
        let snapshot = self
            .ports
            .store
            .contract(&principal, case, contract)?
            .ok_or(Error::NotFound)?;
        Relation::of(&principal, &snapshot, None).read_contract()?;
        if snapshot.root.case != case || snapshot.root.id != contract {
            return Err(Error::Inconsistent.into());
        }
        let view = ContractView::new(snapshot, self.now().unix_timestamp());
        self.reauthenticate(token, &principal)?;
        Ok(view)
    }

    /// The requests designating the caller as Client or firm signer.
    pub fn my_requests(&self, token: &str) -> Result<Vec<RequestView>, ApplicationError> {
        let principal = self.principal(token)?;
        let now = self.now().unix_timestamp();
        let mut views = Vec::new();
        for snapshot in self.ports.store.party_contracts(&principal)? {
            let view = ContractView::new(snapshot, now);
            for request in &view.snapshot.requests {
                let fields = request.request.fields();
                if fields.client == principal.id || fields.firm_signer == principal.id {
                    let status = view.status(request.id()).ok_or(Error::Inconsistent)?;
                    views.push(RequestView {
                        contract: view.clone(),
                        request: request.id(),
                        status,
                    });
                }
            }
        }
        self.reauthenticate(token, &principal)?;
        Ok(views)
    }

    pub fn request(&self, token: &str, request: Uuid) -> Result<RequestView, ApplicationError> {
        let principal = self.principal(token)?;
        let snapshot = self.by_request(&principal, request)?;
        let record = snapshot.request(request).ok_or(Error::Inconsistent)?;
        Relation::of(&principal, &snapshot, Some(record)).read_request()?;
        let contract = ContractView::new(snapshot, self.now().unix_timestamp());
        let status = contract.status(request).ok_or(Error::Inconsistent)?;
        self.reauthenticate(token, &principal)?;
        Ok(RequestView {
            contract,
            request,
            status,
        })
    }

    /// The receipt of a create or abandon operation of `case`.
    pub fn contract_receipt(
        &self,
        token: &str,
        case: CaseId,
        operation: Uuid,
    ) -> Result<ContractReceipt, ApplicationError> {
        let principal = self.principal(token)?;
        let receipt = self
            .find_receipt(&principal, operation)?
            .filter(|receipt| receipt.case == case && receipt.event.request.is_none())
            .ok_or(Error::NotFound)?;
        self.reauthenticate(token, &principal)?;
        Ok(receipt)
    }

    /// The receipt of a mutation of `request`.
    pub fn request_receipt(
        &self,
        token: &str,
        request: Uuid,
        operation: Uuid,
    ) -> Result<ContractReceipt, ApplicationError> {
        let principal = self.principal(token)?;
        let receipt = self
            .find_receipt(&principal, operation)?
            .filter(|receipt| receipt.event.request == Some(request))
            .ok_or(Error::NotFound)?;
        self.reauthenticate(token, &principal)?;
        Ok(receipt)
    }
}
