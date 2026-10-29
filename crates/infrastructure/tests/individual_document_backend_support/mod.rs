pub mod processor;
use crate::signing_binding_support::{self as binding, Clock, Fixture, TOKEN};
use application::{
    document_signing::*,
    documents::{CaseDocumentStore, DocumentRecord},
    identity::Principal,
    ApplicationError,
};
use domain::{cases::CaseId, clock::Clock as _, crypto::DocumentVersionRef};
use infrastructure::{
    certificates::InternalRsaDocumentSigningVerifier, PostgresCaseDocumentStore,
    PostgresIndividualSigningStore, RingSha256Hasher,
};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub struct Harness {
    pub db: Fixture,
    pub clock: Arc<Clock>,
    pub actor: Principal,
    pub record: DocumentRecord,
    pub binding_id: Uuid,
}
impl Harness {
    pub fn new() -> Option<Self> {
        let (mut db, clock, actor) = binding::fixture()?;
        db.admin
            .execute(
                "INSERT INTO case_memberships(case_id,user_id) VALUES($1,$2)",
                &[&db.case.as_uuid(), &actor.id.as_uuid()],
            )
            .unwrap();
        let binding_id = Uuid::new_v4();
        binding::register(
            &binding::service(binding::store(&db, &clock), &clock, actor.clone()),
            binding_id,
        );
        let record = processor::processor()
            .prepare("reviewed.pdf", crate::individual_signing_fixture::PDF)
            .unwrap();
        PostgresCaseDocumentStore::open(&db.runtime_url)
            .unwrap()
            .insert(actor.id, db.case, record.clone(), clock.now())
            .unwrap();
        Some(Self {
            db,
            clock,
            actor,
            record,
            binding_id,
        })
    }
    pub fn reference(&self) -> DocumentVersionRef {
        DocumentVersionRef {
            id: self.record.id,
            version: self.record.version,
        }
    }
    pub fn store(&self) -> Arc<PostgresIndividualSigningStore> {
        Arc::new(
            PostgresIndividualSigningStore::open(&self.db.runtime_url, self.clock.clone()).unwrap(),
        )
    }
    pub fn service(&self, store: Arc<dyn IndividualSigningStore>) -> DocumentSigningService {
        DocumentSigningService::new(DocumentSigningPorts {
            identity: Arc::new(binding::FixedIdentity(self.actor.clone())),
            bindings: binding::store(&self.db, &self.clock),
            documents: Arc::new(PostgresCaseDocumentStore::open(&self.db.runtime_url).unwrap()),
            store,
            verifier: Arc::new(InternalRsaDocumentSigningVerifier),
            hasher: Arc::new(RingSha256Hasher),
            clock: self.clock.clone(),
            processor: Arc::new(processor::processor()),
            formats: Arc::new(processor::Pdf),
            manifest_timestamps: Arc::new(processor::manifest_timestamps()),
        })
    }
    pub fn command(&self, operation: Uuid) -> VerifiedDocumentSigning {
        let capture = Arc::new(Capture(Mutex::new(None)));
        let service = self.service(capture.clone());
        let pdf = crate::individual_signing_fixture::sign(crate::individual_signing_fixture::PDF);
        let prepared = service
            .prepare(TOKEN, operation, self.db.case, self.reference(), &pdf)
            .unwrap();
        let signed = crate::individual_signing_fixture::sign(&prepared.consent().canonical_bytes());
        assert!(service
            .submit(TOKEN, prepared.consent(), &pdf, &signed)
            .is_err());
        let result = capture.0.lock().unwrap().take().unwrap();
        result
    }
    pub fn snapshot(&mut self) -> serde_json::Value {
        self.db.admin.query_one("SELECT jsonb_build_object(
            'documents',(SELECT jsonb_agg(to_jsonb(d) ORDER BY id,version) FROM documents d),
            'receipts',(SELECT jsonb_agg(to_jsonb(r) ORDER BY operation_id) FROM individual_document_signatures r),
            'manifests',(SELECT jsonb_agg(to_jsonb(m) ORDER BY operation_id) FROM individual_document_manifests m),
            'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))", &[]).unwrap().get(0)
    }
    pub fn mutation_counts(&mut self) -> (i64, i64) {
        let row = self
            .db
            .admin
            .query_one(
                "SELECT (SELECT count(*) FROM individual_document_signatures),
            (SELECT count(*) FROM audit_events WHERE action='document.individually_signed')",
                &[],
            )
            .unwrap();
        (row.get(0), row.get(1))
    }
}
struct Capture(Mutex<Option<VerifiedDocumentSigning>>);
impl IndividualSigningStore for Capture {
    fn verification_context(
        &self,
        _: &Principal,
        _: CaseId,
        _: DocumentVersionRef,
    ) -> Result<SigningVerificationContext, ApplicationError> {
        unreachable!()
    }
    fn find(
        &self,
        _: &Principal,
        _: CaseId,
        _: DocumentVersionRef,
        _: Uuid,
    ) -> Result<Option<SigningReceipt>, ApplicationError> {
        Ok(None)
    }
    fn commit(&self, command: VerifiedDocumentSigning) -> Result<SigningCommit, ApplicationError> {
        self.0.lock().unwrap().replace(command);
        Err(ApplicationError::Port("captured signing mutation".into()))
    }
}
