//! Bilateral contracts in PostgreSQL. Reads return whole stored contracts
//! with the caller's scope; every write commits its audit rows, events,
//! request, signatures, completion and notices in one READ COMMITTED
//! transaction under the audited mutation lock and the contract lock, and
//! maps a storage refusal to its contract error. Rationale:
//! docs/adr/0086-bilateral-contract-signing.md.
mod candidates;
mod decode;
mod errors;
mod listing;
mod read;
mod receipt;
mod write;
mod write_rows;
mod write_steps;

use application::{
    contracts::{
        CaseContractsListing, ContractCommand, ContractCommit, ContractRead, ContractReceipt,
        ContractSnapshot, ContractStore, RequestCandidates,
    },
    documents::DocumentRecord,
    identity::Principal,
    ApplicationError,
};
use domain::{cases::CaseId, clock::OffsetDateTime, identity::UserId};
use postgres::Client;
use std::sync::{Mutex, MutexGuard};
use uuid::Uuid;

pub(crate) use errors::port;

/// The advisory lock every contract and signing write takes.
pub(crate) const CONTRACT_LOCK: i64 = 280_603_412_820;

pub struct PostgresContractStore {
    client: Mutex<Client>,
}

impl PostgresContractStore {
    /// Opens validated runtime state without modifying its schema or privileges.
    pub fn open(
        source: &(impl crate::PostgresConnectionSource + ?Sized),
    ) -> Result<Self, ApplicationError> {
        let mut client = crate::postgres::open(source)?;
        client
            .query_one(
                "SELECT pg_catalog.set_config('search_path',
            pg_catalog.format('pg_catalog,%I,pg_temp',n.nspname),false)
            FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON n.oid=c.relnamespace
            WHERE c.oid='contracts'::pg_catalog.regclass",
                &[],
            )
            .map_err(port)?;
        Ok(Self {
            client: Mutex::new(client),
        })
    }

    fn client(&self) -> Result<MutexGuard<'_, Client>, ApplicationError> {
        self.client.lock().map_err(|_| errors::inconsistent())
    }
}

impl ContractStore for PostgresContractStore {
    fn case_contracts(
        &self,
        principal: &Principal,
        case: CaseId,
    ) -> Result<CaseContractsListing, ApplicationError> {
        listing::case_contracts(&mut *self.client()?, principal, case)
    }

    fn contract(
        &self,
        principal: &Principal,
        case: CaseId,
        contract: Uuid,
    ) -> Result<Option<ContractSnapshot>, ApplicationError> {
        let snapshot = read::snapshot(&mut *self.client()?, Some(principal), contract)?;
        Ok(snapshot.filter(|snapshot| snapshot.root.case == case))
    }

    fn contract_by_request(
        &self,
        principal: &Principal,
        request: Uuid,
    ) -> Result<Option<ContractSnapshot>, ApplicationError> {
        let mut client = self.client()?;
        let Some(contract) = read::contract_of(&mut *client, request)? else {
            return Ok(None);
        };
        read::snapshot(&mut *client, Some(principal), contract)
    }

    fn party_contracts(
        &self,
        principal: &Principal,
    ) -> Result<Vec<ContractSnapshot>, ApplicationError> {
        listing::party_contracts(&mut *self.client()?, principal)
    }

    fn receipt(
        &self,
        principal: &Principal,
        operation: Uuid,
    ) -> Result<Option<ContractReceipt>, ApplicationError> {
        let mut client = self.client()?;
        let Some(receipt) = receipt::load(&mut *client, operation)? else {
            return Ok(None);
        };
        let visible = read::snapshot(&mut *client, Some(principal), receipt.contract)?.is_some_and(
            |snapshot| {
                snapshot.case_scope
                    || receipt
                        .event
                        .request
                        .and_then(|id| snapshot.request(id))
                        .is_some_and(|request| {
                            let fields = request.request.fields();
                            fields.client == principal.id || fields.firm_signer == principal.id
                        })
            },
        );
        Ok(visible.then_some(receipt))
    }

    fn request_candidates(
        &self,
        principal: &Principal,
        contract: Uuid,
        client_email: &str,
        firm_signer: UserId,
    ) -> Result<RequestCandidates, ApplicationError> {
        candidates::load(
            &mut *self.client()?,
            principal,
            contract,
            client_email,
            firm_signer,
        )
    }

    fn request_document(
        &self,
        principal: &Principal,
        request: Uuid,
    ) -> Result<DocumentRecord, ApplicationError> {
        read::request_document(&mut *self.client()?, principal, request)
    }

    fn commit(&self, command: ContractCommand) -> Result<ContractCommit, ApplicationError> {
        write::commit(&mut *self.client()?, command)
    }

    fn record_read(
        &self,
        principal: &Principal,
        request: Uuid,
        read: ContractRead,
        at: OffsetDateTime,
    ) -> Result<(), ApplicationError> {
        write::record_read(&mut *self.client()?, principal, request, read, at)
    }

    fn exhausted(&self, at: i64, limit: usize) -> Result<Vec<ContractSnapshot>, ApplicationError> {
        listing::exhausted(&mut *self.client()?, at, limit)
    }
}
