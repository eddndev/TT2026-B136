//! The contract service over real PostgreSQL, real certificates, a real
//! local time-stamping authority and a clock that follows the system clock
//! plus an offset, because the authority stamps the real time:
//! docs/adr/0086-bilateral-contract-signing.md.
use crate::contract_storage_support::{sign_as_client, Harness};
use crate::individual_document_backend_support::processor;
use crate::signing_binding_support::{FixedIdentity, TOKEN};
use application::{
    contracts::*,
    document_signing::{ManifestTimestamping, TokenFacts, TsaAuthority},
    identity::Principal,
    ApplicationError,
};
use domain::{
    clock::{Clock, OffsetDateTime},
    crypto::Sha256Digest,
};
use infrastructure::{
    certificates::InternalRsaDocumentSigningVerifier, PostgresCaseDocumentStore,
    PostgresContractStore, PostgresSigningBindingStore, RingSha256Hasher, StoredZipWriter,
};
use std::sync::{
    atomic::{AtomicBool, AtomicI64, Ordering},
    Arc, OnceLock,
};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub const DAY: i64 = 86_400;

/// The system clock shifted by a settable number of seconds.
#[derive(Default)]
pub struct LiveClock(AtomicI64);
impl LiveClock {
    pub fn shift(&self, seconds: i64) {
        self.0.fetch_add(seconds, Ordering::SeqCst);
    }
}
impl Clock for LiveClock {
    fn now(&self) -> OffsetDateTime {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as i128;
        OffsetDateTime::from_unix_timestamp_nanos(
            nanos + i128::from(self.0.load(Ordering::SeqCst)) * 1_000_000_000,
        )
        .unwrap()
    }
}

/// The local authority, failing every request while `down` is set.
pub struct Authority {
    inner: infrastructure::ManifestTimestampAuthority,
    pub down: AtomicBool,
}
impl ManifestTimestamping for Authority {
    fn facts(&self, token: &[u8], digest: &Sha256Digest) -> Result<TokenFacts, ApplicationError> {
        self.inner.facts(token, digest)
    }
    fn stamp(
        &self,
        digest: &Sha256Digest,
        anchor: &[u8],
        authority: &TsaAuthority<'_>,
    ) -> Result<TokenFacts, ApplicationError> {
        if self.down.load(Ordering::SeqCst) {
            return Err(ApplicationError::TimestampEvidenceRejected(
                "authority is down".into(),
            ));
        }
        self.inner.stamp(digest, anchor, authority)
    }
}

/// Stores that every service of one test shares, as one process would.
struct Stores {
    contract: Arc<PostgresContractStore>,
    bindings: Arc<PostgresSigningBindingStore>,
    incidents: Arc<PostgresCaseDocumentStore>,
}

pub struct Live {
    pub test: Harness,
    pub clock: Arc<LiveClock>,
    pub authority: Arc<Authority>,
    pub contract: Uuid,
    pub request: Uuid,
    // Each open validates the whole catalog (about a second), so services
    // reuse these; `store()` and `opens` remain fresh opens. No contract
    // service test calls a service from several threads.
    stores: OnceLock<Stores>,
}

impl Live {
    /// Parties bound and a pending PDF, all registered at the current time.
    pub fn new() -> Option<Self> {
        let issued = crate::individual_signing_fixture::trust()
            .inspection
            .crl_this_update;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let test = Harness::without_contract_at((now - 1).max(issued))?;
        Some(Self {
            test,
            clock: Arc::new(LiveClock::default()),
            authority: Arc::new(Authority {
                inner: processor::manifest_timestamps(),
                down: AtomicBool::new(false),
            }),
            contract: Uuid::nil(),
            request: Uuid::nil(),
            stores: OnceLock::new(),
        })
    }

    fn stores(&self) -> &Stores {
        self.stores.get_or_init(|| {
            let url = &self.test.individual.db.runtime_url;
            Stores {
                contract: self.store(),
                bindings: Arc::new(
                    PostgresSigningBindingStore::open(url, self.clock.clone()).unwrap(),
                ),
                incidents: Arc::new(PostgresCaseDocumentStore::open(url).unwrap()),
            }
        })
    }

    pub fn store(&self) -> Arc<PostgresContractStore> {
        Arc::new(PostgresContractStore::open(&self.test.individual.db.runtime_url).unwrap())
    }

    pub fn service(&self, principal: &Principal) -> ContractService {
        let stores = self.stores();
        ContractService::new(ContractPorts {
            identity: Arc::new(FixedIdentity(principal.clone())),
            store: stores.contract.clone(),
            bindings: stores.bindings.clone(),
            verifier: Arc::new(InternalRsaDocumentSigningVerifier),
            timestamps: self.authority.clone(),
            hasher: Arc::new(RingSha256Hasher),
            clock: self.clock.clone(),
            processor: Arc::new(processor::processor()),
            formats: Arc::new(processor::Pdf),
            incidents: stores.incidents.clone(),
            archiver: Arc::new(StoredZipWriter),
            notices: NoticeSettings {
                from: "qadra@example.test".into(),
                public_url: Some("https://qadra.example.test".into()),
            },
        })
    }

    pub fn firm(&self) -> Principal {
        self.test.firm()
    }

    pub fn client(&self) -> Principal {
        self.test.client.clone()
    }

    /// Creates the contract and one seven-day request.
    pub fn requested(mut self) -> Self {
        let firm = self.firm();
        let created = self
            .service(&firm)
            .create(
                TOKEN,
                self.test.individual.db.case,
                Uuid::new_v4(),
                self.test.individual.record.id,
                "Contrato de servicios",
            )
            .unwrap();
        self.contract = created.contract;
        self.request = Uuid::new_v4();
        self.service(&firm)
            .create_request(
                TOKEN,
                self.test.individual.db.case,
                self.contract,
                &self.new_request(1),
            )
            .unwrap();
        self
    }

    pub fn new_request(&self, expected_revision: u32) -> NewRequest {
        NewRequest {
            request: self.request,
            operation: Uuid::new_v4(),
            expected_revision,
            version: self.test.individual.record.version.get(),
            client_email: self.client().email,
            firm_signer: self.firm().id,
            ttl_days: 7,
            ceremony_reference: "HF-2026-001".into(),
            subject: None,
        }
    }

    /// The prepared and signed statement of `principal`.
    pub fn signed(&self, principal: &Principal, operation: Uuid) -> SubmittedSignature {
        let prepared = self
            .service(principal)
            .prepare(TOKEN, self.request, operation)
            .unwrap();
        let bytes = prepared.statement.canonical_bytes();
        let signature = if principal.id == self.client().id {
            sign_as_client(&bytes)
        } else {
            crate::individual_signing_fixture::sign(&bytes)
        };
        SubmittedSignature {
            operation,
            expected_revision: prepared.revision,
            statement: bytes.to_vec(),
            signature: signature.as_bytes().to_vec(),
        }
    }

    pub fn sign(
        &self,
        principal: &Principal,
        submitted: &SubmittedSignature,
    ) -> Result<ContractReceipt, ApplicationError> {
        self.service(principal).sign(TOKEN, self.request, submitted)
    }

    /// Rows of one contract table; every write appends to these.
    pub fn count(&mut self, table: &str) -> i64 {
        self.test
            .db()
            .admin
            .query_one(&format!("SELECT count(*) FROM {table}"), &[])
            .unwrap()
            .get(0)
    }
}

pub fn contract_error<T: std::fmt::Debug>(
    result: Result<T, ApplicationError>,
) -> ContractWorkflowError {
    match result {
        Err(ApplicationError::Contract(error)) => error,
        other => panic!("expected a contract error, got {other:?}"),
    }
}
