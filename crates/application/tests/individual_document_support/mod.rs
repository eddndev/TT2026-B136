mod processor;

use crate::{case_document_support::MockStore as Documents, document_signing_support as bindings};
use application::{
    document_signing::{
        DocumentSigningPorts, DocumentSigningService, IndividualSigningStore, SigningCommit,
        SigningReceipt, SigningVerificationContext, VerifiedDocumentSigning,
    },
    documents::{DocumentAction, DocumentProcessor, DocumentRecord, VersionSelection},
    identity::Principal,
    ApplicationError,
};
use domain::{
    cases::CaseId,
    clock::{Clock, OffsetDateTime},
    crypto::{
        CredentialCheck, DocumentHasher, DocumentId, DocumentVersion, DocumentVersionRef, Signature,
    },
    document_signing::DocumentConsent,
};
use mockall::mock;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub const PDF: &[u8] = b"%PDF-1.7 exact selected PDF";
pub fn case() -> CaseId {
    CaseId::from_uuid(Uuid::from_u128(70))
}
pub fn operation() -> Uuid {
    Uuid::from_u128(60)
}
pub fn reference() -> DocumentVersionRef {
    DocumentVersionRef {
        id: DocumentId::from_uuid(Uuid::from_u128(80)),
        version: DocumentVersion::initial(),
    }
}
pub fn pdf_signature() -> Signature {
    Signature::from_bytes(vec![7; 384]).unwrap()
}
pub fn consent_signature() -> Signature {
    Signature::from_bytes(vec![8; 384]).unwrap()
}
pub fn consent() -> DocumentConsent {
    DocumentConsent::new(
        bindings::account(),
        *bindings::statement().material(),
        operation(),
        case(),
        reference(),
        bindings::Hasher.hash_bytes(PDF),
        bindings::Hasher.hash_bytes(pdf_signature().as_bytes()),
    )
    .unwrap()
}

mock! { pub Store {} impl IndividualSigningStore for Store {
    fn find(&self, principal: &Principal, case: CaseId, reference: DocumentVersionRef, operation: Uuid)
        -> Result<Option<SigningReceipt>, ApplicationError>;
    fn commit(&self, verified: VerifiedDocumentSigning) -> Result<SigningCommit, ApplicationError>;
    fn verification_context(&self, principal: &Principal, case: CaseId, reference: DocumentVersionRef)
        -> Result<SigningVerificationContext, ApplicationError>;
} }

pub struct State {
    pub record: DocumentRecord,
    pub found: Option<SigningReceipt>,
    pub commits: usize,
    pub tsa_calls: usize,
    pub format_calls: usize,
    pub docx: bool,
    pub bad_timestamp: bool,
    pub timeout: bool,
    pub deny_document: bool,
    pub corrupt_receipt: bool,
    pub change_on_find: bool,
    pub concurrent_receipt: Option<SigningReceipt>,
}
struct ClockView(Arc<Mutex<bindings::State>>);
impl Clock for ClockView {
    fn now(&self) -> OffsetDateTime {
        bindings::at(self.0.lock().unwrap().now)
    }
}
pub struct Harness {
    pub binding: bindings::Harness,
    pub state: Arc<Mutex<State>>,
    pub documents: Documents,
    pub store: MockStore,
    pub processor: Arc<DocumentProcessor>,
}
impl Harness {
    pub fn new() -> Self {
        let binding = bindings::Harness::new();
        binding.state.lock().unwrap().found = Some(bindings::receipt());
        let record = crate::crypto::processor()
            .prepare_version(reference().id, reference().version, "selected.pdf", PDF)
            .unwrap();
        let state = Arc::new(Mutex::new(State {
            record,
            found: None,
            commits: 0,
            tsa_calls: 0,
            format_calls: 0,
            docx: false,
            bad_timestamp: false,
            timeout: false,
            deny_document: false,
            corrupt_receipt: false,
            change_on_find: false,
            concurrent_receipt: None,
        }));
        let processor = Arc::new(processor::processor(state.clone()));
        let mut documents = Documents::new();
        let observed = state.clone();
        documents
            .expect_load()
            .returning(move |actor, found_case, document, selection, action| {
                assert_eq!(actor, bindings::principal().id);
                assert_eq!(
                    (found_case, document, selection, action),
                    (
                        case(),
                        reference().id,
                        VersionSelection::Exact(reference().version),
                        DocumentAction::Seal
                    )
                );
                let state = observed.lock().unwrap();
                if state.deny_document {
                    return Err(ApplicationError::PermissionDenied);
                }
                Ok(state.record.clone())
            });
        let mut store = MockStore::new();
        let observed = state.clone();
        let authority = binding.state.clone();
        store
            .expect_find()
            .returning(move |actor, found_case, document, op| {
                assert_eq!(actor, &bindings::principal());
                assert_eq!(
                    (found_case, document, op),
                    (case(), reference(), operation())
                );
                let state = observed.lock().unwrap();
                if state.change_on_find {
                    authority.lock().unwrap().principal.as_mut().unwrap().email =
                        "changed@example.test".into();
                }
                Ok(state.found.clone())
            });
        let observed = state.clone();
        store.expect_commit().returning(move |verified| {
            assert_eq!(verified.principal(), &bindings::principal());
            assert_eq!(verified.account(), &bindings::account());
            assert_eq!(verified.trust(), &bindings::trust());
            assert_eq!(verified.binding(), &bindings::receipt());
            assert_eq!(verified.consent(), &consent());
            assert_eq!(verified.check().signature, consent_signature());
            assert_eq!(verified.not_before(), bindings::at(1000));
            let evidence = verified.record().evidence.clone().unwrap();
            assert_eq!(evidence.signature, pdf_signature().as_bytes());
            assert_eq!(
                evidence.individual.as_ref().unwrap().consent_signature,
                consent_signature().as_bytes()
            );
            let mut receipt = SigningReceipt {
                consent: *verified.consent(),
                evidence,
                accepted_at: bindings::at(1000),
            };
            let mut state = observed.lock().unwrap();
            state.commits += 1;
            if state.corrupt_receipt {
                receipt.evidence.signature[0] ^= 1;
            }
            state.found = Some(receipt.clone());
            Ok(match state.concurrent_receipt.clone() {
                Some(original) => SigningCommit::Existing(original),
                None => SigningCommit::Applied(receipt),
            })
        });
        Self {
            binding,
            state,
            documents,
            store,
            processor,
        }
    }
    pub fn verify_with(
        &mut self,
        effect: impl Fn(&mut CredentialCheck, &Arc<Mutex<bindings::State>>) + Send + Sync + 'static,
    ) {
        let observed = self.binding.state.clone();
        self.binding
            .verifier
            .expect_verify_consent()
            .times(1)
            .returning(move |value, certificate, pdf, proof, trust, now| {
                assert_eq!(value, &consent());
                assert_eq!(certificate, bindings::certificate().der);
                assert_eq!(pdf, &pdf_signature());
                assert_eq!(proof, &consent_signature());
                let mut check = bindings::check(&bindings::statement(), proof, trust, now);
                check.statement_digest = bindings::Hasher.hash_bytes(&value.canonical_bytes());
                effect(&mut check, &observed);
                Ok(check)
            });
    }
    pub fn service(self) -> DocumentSigningService {
        DocumentSigningService::new(DocumentSigningPorts {
            identity: Arc::new(self.binding.identity),
            bindings: Arc::new(self.binding.store),
            documents: Arc::new(self.documents),
            store: Arc::new(self.store),
            verifier: Arc::new(self.binding.verifier),
            hasher: Arc::new(bindings::Hasher),
            clock: Arc::new(ClockView(self.binding.state)),
            processor: self.processor,
            formats: Arc::new(processor::Format(self.state)),
        })
    }
}
