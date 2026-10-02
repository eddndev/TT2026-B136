//! Own report HTTP and background workflow adapters outside the async runtime.
use anyhow::Context;
use application::{
    case_reports::{CaseReportService, CaseReportWorker, CaseReportWorkerRun, CaseReportWorkflow},
    identity::IdentityWorkflow,
    ApplicationError,
};
use infrastructure::{
    case_report_isolation::IsolatedCaseReportRenderer, CaseReportEnvelopeProtector,
    EnvelopeKeyManager, PostgresCaseReportStore, RingAesGcmCipher, RingSha256Hasher, SystemClock,
};
use std::sync::Arc;
use zeroize::Zeroizing;

pub(crate) type ReportConsumer =
    Arc<dyn Fn() -> Result<CaseReportWorkerRun, ApplicationError> + Send + Sync>;

pub(crate) struct ReportComponents {
    pub workflow: Arc<dyn CaseReportWorkflow>,
    pub consumer: ReportConsumer,
}

pub(crate) fn open(
    database_url: &(impl infrastructure::PostgresConnectionSource + ?Sized),
    identity: Arc<dyn IdentityWorkflow>,
    kek: Zeroizing<Vec<u8>>,
) -> anyhow::Result<ReportComponents> {
    let executable = std::env::current_exe().context("cannot locate report render worker")?;
    let renderer = Arc::new(
        IsolatedCaseReportRenderer::new(executable)
            .context("cannot configure isolated report rendering")?,
    );
    let hasher = Arc::new(RingSha256Hasher::new());
    let clock = Arc::new(SystemClock::new());
    let protector = Arc::new(
        CaseReportEnvelopeProtector::new(
            Arc::new(RingAesGcmCipher::new()),
            Arc::new(EnvelopeKeyManager::new()),
            kek.to_vec(),
        )
        .context("cannot initialize report encryption")?,
    );
    let store = Arc::new(
        PostgresCaseReportStore::open(database_url, hasher.clone(), clock.clone(), protector)
            .context("cannot open PostgreSQL report store")?,
    );
    let workflow = Arc::new(CaseReportService::new(
        store.clone(),
        identity,
        hasher.clone(),
        clock.clone(),
    ));
    let worker = CaseReportWorker::new(store, renderer, hasher, clock);
    Ok(ReportComponents {
        workflow,
        consumer: Arc::new(move || worker.run_next()),
    })
}
