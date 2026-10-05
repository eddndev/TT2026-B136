//! Compose ordinary hearing scheduling and declared results with shared admission.
use anyhow::Context;
use application::{
    documents::{DocumentFormatBatchValidator, DocumentProcessor},
    hearing_results::HearingResultService,
    hearings::HearingService,
    identity::IdentityWorkflow,
};
use infrastructure::{
    PostgresHearingResultStore, PostgresHearingStore, RingSha256Hasher, SystemClock,
};
use std::sync::Arc;

pub(super) fn open(
    database: &(impl infrastructure::PostgresConnectionSource + ?Sized),
    identity: Arc<dyn IdentityWorkflow>,
    processor: Arc<DocumentProcessor>,
    validator: Arc<dyn DocumentFormatBatchValidator>,
) -> anyhow::Result<(HearingService, HearingResultService)> {
    let hearing_hasher = Arc::new(RingSha256Hasher::new());
    let hearing_clock = Arc::new(SystemClock::new());
    let hearings = HearingService::new(
        Arc::new(
            PostgresHearingStore::open(database, hearing_hasher.clone(), hearing_clock.clone())
                .context("cannot open PostgreSQL hearing store")?,
        ),
        identity.clone(),
        processor.clone(),
        validator.clone(),
        hearing_hasher,
        hearing_clock,
    );
    let result_hasher = Arc::new(RingSha256Hasher::new());
    let result_clock = Arc::new(SystemClock::new());
    let hearing_results = HearingResultService::new(
        Arc::new(
            PostgresHearingResultStore::open(database, result_hasher.clone(), result_clock.clone())
                .context("cannot open PostgreSQL hearing result store")?,
        ),
        identity,
        processor,
        validator,
        result_hasher,
        result_clock,
    );
    Ok((hearings, hearing_results))
}
