//! Compose individual public proofs and atomic exact-version signing receipts.

use anyhow::Context;
use application::{
    document_signing::{
        DocumentSigningPorts, DocumentSigningService, SigningBindingPorts, SigningBindingService,
    },
    documents::{CaseDocumentStore, DocumentFormatBatchValidator, DocumentProcessor},
    identity::IdentityWorkflow,
};
use infrastructure::{
    certificates::InternalRsaDocumentSigningVerifier, PostgresConnectionSource,
    PostgresIndividualSigningStore, PostgresSigningBindingStore, RingSha256Hasher, SystemClock,
};
use std::sync::Arc;

pub(super) fn open(
    database: &(impl PostgresConnectionSource + ?Sized),
    identity: Arc<dyn IdentityWorkflow>,
    documents: Arc<dyn CaseDocumentStore>,
    processor: Arc<DocumentProcessor>,
    formats: Arc<dyn DocumentFormatBatchValidator>,
) -> anyhow::Result<web::IndividualSigningHttp> {
    let clock = Arc::new(SystemClock::new());
    let hasher = Arc::new(RingSha256Hasher::new());
    let verifier = Arc::new(InternalRsaDocumentSigningVerifier::new());
    let bindings = Arc::new(
        PostgresSigningBindingStore::open(database, clock.clone())
            .context("cannot open PostgreSQL individual certificate binding store")?,
    );
    let receipts = Arc::new(
        PostgresIndividualSigningStore::open(database, clock.clone())
            .context("cannot open PostgreSQL individual document signature store")?,
    );
    Ok(web::IndividualSigningHttp {
        bindings: Arc::new(SigningBindingService::new(SigningBindingPorts {
            identity: identity.clone(),
            store: bindings.clone(),
            verifier: verifier.clone(),
            hasher: hasher.clone(),
            clock: clock.clone(),
        })),
        documents: Arc::new(DocumentSigningService::new(DocumentSigningPorts {
            identity,
            bindings,
            documents,
            store: receipts,
            verifier,
            hasher,
            clock,
            processor,
            formats,
        })),
    })
}
