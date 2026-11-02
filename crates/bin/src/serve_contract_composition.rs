//! Compose bilateral contract signing: the HTTP service, the expiry closer
//! that always runs, and notice e-mail only when it is enabled. Each part
//! holds its own PostgreSQL connection. See
//! docs/adr/0086-bilateral-contract-signing.md.
use crate::{serve_contract_config::ContractSettings, serve_contract_runtime::ContractConsumer};
use anyhow::Context;
use application::{
    contracts::{ContractExpiryService, ContractNoticeWorker, ContractPorts, ContractService},
    document_integrity::DocumentIntegrityStore,
    documents::{DocumentFormatBatchValidator, DocumentProcessor},
    identity::IdentityWorkflow,
};
use infrastructure::{
    certificates::InternalRsaDocumentSigningVerifier, LocalOpensslTsa, ManifestTimestampAuthority,
    PostgresConnectionSource, PostgresContractStore, PostgresSigningBindingStore,
    ResendCaseReportEmailDelivery, Rfc3161Verifier, RingSha256Hasher, StoredZipWriter, SystemClock,
};
use std::{sync::Arc, time::Duration};

pub(crate) struct ContractComponents {
    pub http: web::ContractHttp,
    pub consumer: ContractConsumer,
}

pub(crate) fn open(
    args: &crate::serve_args::ServeArgs,
    database: &(impl PostgresConnectionSource + ?Sized),
    identity: Arc<dyn IdentityWorkflow>,
    processor: Arc<DocumentProcessor>,
    formats: Arc<dyn DocumentFormatBatchValidator>,
    incidents: Arc<dyn DocumentIntegrityStore>,
    settings: ContractSettings,
) -> anyhow::Result<ContractComponents> {
    let store =
        || PostgresContractStore::open(database).context("cannot open PostgreSQL contract store");
    let clock = Arc::new(SystemClock::new());
    let service = ContractService::new(ContractPorts {
        identity,
        store: Arc::new(store()?),
        bindings: Arc::new(
            PostgresSigningBindingStore::open(database, clock.clone())
                .context("cannot open PostgreSQL contract binding store")?,
        ),
        verifier: Arc::new(InternalRsaDocumentSigningVerifier::new()),
        // T1, Td and Tf come from the same local authority and anchor as
        // the individual signing manifest token.
        timestamps: Arc::new(ManifestTimestampAuthority::new(
            Box::new(LocalOpensslTsa::new(&args.tsa_config, &args.tsa_dir)),
            Box::new(Rfc3161Verifier::new()),
        )),
        hasher: Arc::new(RingSha256Hasher::new()),
        clock: clock.clone(),
        processor,
        formats,
        incidents,
        archiver: Arc::new(StoredZipWriter::new()),
        notices: settings.notices.clone(),
    });
    let expiry = ContractExpiryService::new(Arc::new(store()?), clock.clone(), settings.notices);
    let notices = match settings.email {
        Some(api_key) => {
            let delivery = Arc::new(ResendCaseReportEmailDelivery::new(
                api_key,
                Duration::from_secs(2),
                Duration::from_secs(5),
            )?);
            let worker = ContractNoticeWorker::new(Arc::new(store()?), delivery, clock);
            Some(Arc::new(move || worker.run_once()) as _)
        }
        None => None,
    };
    Ok(ContractComponents {
        http: web::ContractHttp {
            service: Arc::new(service),
        },
        consumer: ContractConsumer {
            expiry: Arc::new(move || expiry.run_once()),
            notices,
        },
    })
}
