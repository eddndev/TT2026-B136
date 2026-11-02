//! Doubles for the contract service: an in-memory store that records every
//! commit, a binding store, a verifier and a time-stamping authority whose
//! failures and instants each test chooses. No cryptography is checked.
pub mod store;

pub use store::FakeStore;

/// The time-stamping anchor: the synthetic trust root `root` as PEM.
pub const ROOT_PEM: &[u8] = b"-----BEGIN CERTIFICATE-----\ncm9vdA==\n-----END CERTIFICATE-----\n";

pub struct FixedClock(i64);
impl Clock for FixedClock {
    fn now(&self) -> OffsetDateTime {
        at(self.0)
    }
}

pub fn fixed_clock(seconds: i64) -> FixedClock {
    FixedClock(seconds)
}

use crate::case_support::MockIdentity;
use crate::document_signing_support::{self as signing, at, Hasher};
use application::{
    contracts::*,
    credential_trust::CredentialTrustSnapshot,
    document_integrity::{
        DocumentIntegrityIncident, DocumentIntegrityIncidentId, DocumentIntegrityObservation,
        DocumentIntegrityPage, DocumentIntegrityQuery, DocumentIntegrityReceipt,
        DocumentIntegrityStore,
    },
    document_signing::{
        ManifestTimestamping, SigningAccountContext, SigningBindingReceipt, SigningBindingStore,
        SigningWorkflowError, TokenFacts, TsaAuthority, VerifiedSigningBinding,
    },
    documents::{DocumentFormatBatch, DocumentFormatBatchValidator, StageDocumentFormat},
    identity::Principal,
    ApplicationError,
};
use domain::{
    clock::{Clock, OffsetDateTime},
    crypto::{
        ArchiveEntry, ArchiveWriter, CertificateSummary, CredentialCertificate, CredentialCheck,
        CredentialFailure, DocumentHasher, Sha256Digest, Signature,
    },
    document_signing::{
        contract::ContractPartyStatement, SignerAccount, SigningBindingMaterial,
        SigningBindingStatement,
    },
    identity::{Role, UserId},
    DomainError,
};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub const CREATED: i64 = 1000;
pub const DAY: i64 = 86_400;

/// Every choice a test makes and every call it observes.
pub struct World {
    pub now: i64,
    pub principal: Principal,
    pub tsa_failure: bool,
    pub gen_time: i64,
    pub tsa_calls: usize,
    pub verify_calls: usize,
    pub client_revoked: bool,
}

pub fn client() -> Principal {
    Principal {
        id: UserId::from_uuid(Uuid::from_u128(2)),
        email: "client@example.test".into(),
        role: Role::Client,
    }
}

pub fn firm() -> Principal {
    signing::principal()
}

fn client_certificate() -> CredentialCertificate {
    let der = b"synthetic-client-certificate".to_vec();
    CredentialCertificate {
        fingerprint: Hasher.hash_bytes(&der),
        der,
        summary: CertificateSummary {
            subject: "Synthetic Client".into(),
            issuer: "Synthetic Root".into(),
            serial_hex: "2000".into(),
            not_before_unix: 50,
            not_after_unix: 2500,
        },
    }
}

pub fn client_binding() -> SigningBindingReceipt {
    let trust = signing::trust();
    let account = SignerAccount::for_contract(client().id, Role::Client, true, 3, 1).unwrap();
    let statement = SigningBindingStatement::new(
        account,
        client().id,
        SigningBindingMaterial::new(
            trust.deployment_id,
            Uuid::from_u128(45),
            trust.inspection.root_fingerprint,
            client_certificate().fingerprint,
            trust.revision.get(),
        )
        .unwrap(),
    )
    .unwrap();
    let mut check = signing::check(&statement, &signing::signature(), &trust, 1000);
    check.certificate = client_certificate();
    SigningBindingReceipt {
        statement,
        check,
        trust,
        registered_at: at(900),
        withdrawn_at: None,
    }
}

fn binding_of(actor: UserId) -> SigningBindingReceipt {
    if actor == client().id {
        client_binding()
    } else {
        signing::receipt()
    }
}

struct Bindings(Arc<Mutex<World>>);
impl SigningBindingStore for Bindings {
    fn load_context(&self, actor: UserId) -> Result<SigningAccountContext, ApplicationError> {
        let world = self.0.lock().unwrap();
        assert_eq!(actor, world.principal.id);
        Ok(SigningAccountContext {
            principal: world.principal.clone(),
            account: *binding_of(actor).statement.account(),
            trust: Some(signing::trust()),
        })
    }
    fn find(
        &self,
        actor: UserId,
        _: Uuid,
    ) -> Result<Option<SigningBindingReceipt>, ApplicationError> {
        Ok(Some(binding_of(actor)))
    }
    fn find_current(
        &self,
        actor: UserId,
    ) -> Result<Option<SigningBindingReceipt>, ApplicationError> {
        Ok(Some(binding_of(actor)))
    }
    fn commit_binding(
        &self,
        _: VerifiedSigningBinding,
    ) -> Result<SigningBindingReceipt, ApplicationError> {
        unreachable!("contracts never register bindings")
    }
    fn withdraw(
        &self,
        _: &Principal,
        _: Uuid,
        _: SignerAccount,
        _: OffsetDateTime,
    ) -> Result<SigningBindingReceipt, ApplicationError> {
        unreachable!("contracts never withdraw bindings")
    }
}

struct Verifier(Arc<Mutex<World>>);
impl ContractSignatureVerifier for Verifier {
    fn verify_statement(
        &self,
        statement: &ContractPartyStatement,
        certificate: &[u8],
        signature: &Signature,
        trust: &CredentialTrustSnapshot,
        at: i64,
    ) -> Result<CredentialCheck, SigningWorkflowError> {
        self.0.lock().unwrap().verify_calls += 1;
        let binding = binding_of(statement.account().id());
        assert_eq!(certificate, binding.check.certificate.der);
        Ok(CredentialCheck {
            certificate: binding.check.certificate,
            trust: trust.inspection.clone(),
            statement_digest: Hasher.hash_bytes(&statement.canonical_bytes()),
            signature: signature.clone(),
            checked_at: at,
            valid_from: 100,
            valid_until: 2000,
        })
    }
    fn certificate_status(
        &self,
        certificate: &[u8],
        _: &CredentialTrustSnapshot,
        _: i64,
    ) -> Result<(), CredentialFailure> {
        assert_eq!(certificate, client_certificate().der);
        if self.0.lock().unwrap().client_revoked {
            return Err(CredentialFailure::Revoked);
        }
        Ok(())
    }
}

struct Timestamps(Arc<Mutex<World>>);
impl ManifestTimestamping for Timestamps {
    fn facts(&self, token: &[u8], _: &Sha256Digest) -> Result<TokenFacts, ApplicationError> {
        Ok(TokenFacts {
            token: token.to_vec(),
            gen_time: CREATED + 5,
            accuracy_seconds: 1,
        })
    }
    fn stamp(
        &self,
        digest: &Sha256Digest,
        _: &[u8],
        _: &TsaAuthority<'_>,
    ) -> Result<TokenFacts, ApplicationError> {
        let mut world = self.0.lock().unwrap();
        world.tsa_calls += 1;
        if world.tsa_failure {
            return Err(ApplicationError::TimestampEvidenceRejected(
                "synthetic outage".into(),
            ));
        }
        Ok(TokenFacts {
            token: digest.as_bytes().to_vec(),
            gen_time: world.gen_time,
            accuracy_seconds: 1,
        })
    }
}

struct WorldClock(Arc<Mutex<World>>);
impl Clock for WorldClock {
    fn now(&self) -> OffsetDateTime {
        at(self.0.lock().unwrap().now)
    }
}

struct Pdf;
impl DocumentFormatBatchValidator for Pdf {
    fn validate_batch(
        &self,
        batch: &DocumentFormatBatch<'_>,
    ) -> Result<Vec<StageDocumentFormat>, ApplicationError> {
        Ok(vec![StageDocumentFormat::Pdf; batch.inputs().len()])
    }
}

struct NoIncidents;
impl DocumentIntegrityStore for NoIncidents {
    fn record_rejection(
        &self,
        _: &DocumentIntegrityObservation,
    ) -> Result<DocumentIntegrityReceipt, ApplicationError> {
        unreachable!("synthetic content is intact")
    }
    fn list(
        &self,
        _: UserId,
        _: DocumentIntegrityQuery,
        _: OffsetDateTime,
    ) -> Result<DocumentIntegrityPage, ApplicationError> {
        unreachable!()
    }
    fn get(
        &self,
        _: UserId,
        _: DocumentIntegrityIncidentId,
        _: OffsetDateTime,
    ) -> Result<DocumentIntegrityIncident, ApplicationError> {
        unreachable!()
    }
}

struct NoArchive;
impl ArchiveWriter for NoArchive {
    fn write_archive(&self, _: &[ArchiveEntry]) -> Result<Vec<u8>, DomainError> {
        unreachable!("these tests export nothing")
    }
}

pub struct Harness {
    pub world: Arc<Mutex<World>>,
    pub store: Arc<FakeStore>,
}

impl Harness {
    pub fn new(principal: Principal) -> Self {
        Self {
            world: Arc::new(Mutex::new(World {
                now: CREATED + 10,
                principal,
                tsa_failure: false,
                gen_time: CREATED + 5,
                tsa_calls: 0,
                verify_calls: 0,
                client_revoked: false,
            })),
            store: Arc::new(FakeStore::default()),
        }
    }

    pub fn service(&self) -> ContractService {
        let mut identity = MockIdentity::new();
        let world = self.world.clone();
        identity
            .expect_authenticate()
            .returning(move |_| Ok(world.lock().unwrap().principal.clone()));
        let mut processor = crate::crypto::processor_ports();
        processor.hasher = Box::new(Hasher);
        let mut material = crate::crypto::evidence_material();
        material.tsa_chain_pem = Some(ROOT_PEM.to_vec());
        ContractService::new(ContractPorts {
            identity: Arc::new(identity),
            store: self.store.clone(),
            bindings: Arc::new(Bindings(self.world.clone())),
            verifier: Arc::new(Verifier(self.world.clone())),
            timestamps: Arc::new(Timestamps(self.world.clone())),
            hasher: Arc::new(Hasher),
            clock: Arc::new(WorldClock(self.world.clone())),
            processor: Arc::new(
                application::documents::DocumentProcessor::new(
                    processor,
                    material,
                    zeroize::Zeroizing::new(vec![0x44; 32]),
                )
                .unwrap(),
            ),
            formats: Arc::new(Pdf),
            incidents: Arc::new(NoIncidents),
            archiver: Arc::new(NoArchive),
            notices: NoticeSettings {
                from: "qadra@example.test".into(),
                public_url: None,
            },
        })
    }

    pub fn set(&self, change: impl FnOnce(&mut World)) {
        change(&mut self.world.lock().unwrap());
    }

    pub fn calls(&self) -> (usize, usize, usize) {
        let world = self.world.lock().unwrap();
        (world.verify_calls, world.tsa_calls, self.store.commits())
    }
}

pub fn contract_error(
    result: Result<impl std::fmt::Debug, ApplicationError>,
) -> ContractWorkflowError {
    match result {
        Err(ApplicationError::Contract(error)) => error,
        other => panic!("expected a contract error, got {other:?}"),
    }
}

pub fn signing_error(
    result: Result<impl std::fmt::Debug, ApplicationError>,
) -> SigningWorkflowError {
    match result {
        Err(ApplicationError::DocumentSigning(error)) => error,
        other => panic!("expected a signing error, got {other:?}"),
    }
}
