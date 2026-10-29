//! An individual signature accepted against PostgreSQL with a local OpenSSL
//! time-stamping authority issued by the credential trust root, exported
//! twice through the case document service, and validated by the pure
//! verifier that the browser runs. Rationale:
//! `docs/adr/0085-in-browser-evidence-validation.md`.
#![allow(dead_code)]
mod case_administration_support;
mod declaration_fixture;
mod individual_document_backend_support;
mod individual_signing_fixture;
mod signing_binding_support;

use application::{
    documents::{
        AdmittedDocumentFormat, CaseDocumentService, CaseDocumentWorkflow, DocumentUploadAdmission,
    },
    ApplicationError,
};
use domain::clock::Clock as _;
use evidence_verify::{
    policy::{PinnedRoot, Policy, Purposes},
    report::Profile,
    validate, Outcome,
};
use individual_document_backend_support::{processor, Harness};
use infrastructure::PostgresCaseDocumentStore;
use signing_binding_support::{FixedIdentity, TOKEN};
use std::sync::Arc;
use uuid::Uuid;

struct NoUploads;
impl DocumentUploadAdmission for NoUploads {
    fn validate(&self, _: &[u8]) -> Result<AdmittedDocumentFormat, ApplicationError> {
        unreachable!("the export path admits no upload")
    }
}

#[test]
fn an_accepted_individual_signature_exports_a_complete_v2_package() {
    let Some(test) = Harness::new() else {
        return;
    };
    let service = test.service(test.store());
    let pdf = individual_signing_fixture::sign(individual_signing_fixture::PDF);
    let prepared = service
        .prepare(TOKEN, Uuid::new_v4(), test.db.case, test.reference(), &pdf)
        .unwrap();
    let consent = individual_signing_fixture::sign(&prepared.consent().canonical_bytes());
    let receipt = service
        .submit(TOKEN, prepared.consent(), &pdf, &consent)
        .unwrap();
    assert!(receipt.manifest.is_some());

    let documents = CaseDocumentService::new(
        Arc::new(PostgresCaseDocumentStore::open(&test.db.runtime_url).unwrap()),
        Arc::new(FixedIdentity(test.actor.clone())),
        processor::processor().with_individual_verifier(Box::new(
            infrastructure::certificates::InternalRsaDeclarationVerifier::new(),
        )),
        Arc::new(NoUploads),
        test.clock.clone(),
    );
    let first = documents
        .export_evidence(TOKEN, test.db.case, test.record.id)
        .unwrap();
    let second = documents
        .export_evidence(TOKEN, test.db.case, test.record.id)
        .unwrap();
    assert_eq!(first.archive, second.archive, "repeated exports differ");

    let root = individual_signing_fixture::trust().inspection.root_der;
    let policy = Policy::new(
        vec![PinnedRoot {
            sha256: evidence_verify::certificate::sha256(&root),
            purposes: Purposes::ALL,
            label: "credential trust root".to_owned(),
        }],
        test.clock.now().unix_timestamp() + 60,
    );
    let report = validate(&first.archive, &policy);
    assert_eq!(
        report.profile,
        Some(Profile::IndividualSigningV2),
        "{report:#?}"
    );
    assert_eq!(report.overall, Outcome::Checked, "{report:#?}");
    for component in &report.components {
        assert!(
            matches!(component.outcome, Outcome::Checked | Outcome::NotApplicable),
            "{report:#?}"
        );
    }
}
