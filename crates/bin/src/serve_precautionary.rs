//! Compose precautionary appointments, declared decisions and authorized records.
use anyhow::Context;
use application::{
    documents::{DocumentFormatBatchValidator, DocumentProcessor},
    identity::IdentityWorkflow,
    measure_corrections::{
        MeasureAdministrativeReadService, MeasureAdministrativeService, MeasureRecordReadService,
    },
    precautionary_hearings::{
        PrecautionaryContextReadService, PrecautionaryHearingRecordReadService,
        PrecautionaryHearingRecordService,
    },
    precautionary_measures::{MeasureDecisionRecordReadService, MeasureDecisionRecordService},
};
use infrastructure::{
    PostgresMeasureAdministrativeStore, PostgresMeasureDecisionStore,
    PostgresPrecautionaryHearingStore,
};
use std::sync::Arc;

pub(crate) fn open_hearings(
    database: &(impl infrastructure::PostgresConnectionSource + ?Sized),
    identity: Arc<dyn IdentityWorkflow>,
    processor: Arc<DocumentProcessor>,
    validator: Arc<dyn DocumentFormatBatchValidator>,
) -> anyhow::Result<web::PrecautionaryWorkflows> {
    let hasher = Arc::new(infrastructure::RingSha256Hasher);
    let clock = Arc::new(infrastructure::SystemClock::new());
    let store = Arc::new(
        PostgresPrecautionaryHearingStore::open(database, hasher.clone(), clock.clone())
            .context("cannot open PostgreSQL precautionary hearing store")?,
    );
    let context = Arc::new(PrecautionaryContextReadService::new(
        store.clone(),
        identity.clone(),
        hasher.clone(),
        clock.clone(),
    ));
    let writes = Arc::new(PrecautionaryHearingRecordService::new(
        store.clone(),
        identity.clone(),
        processor.clone(),
        validator.clone(),
        hasher.clone(),
        clock.clone(),
    ));
    let reads = Arc::new(PrecautionaryHearingRecordReadService::new(
        store,
        identity.clone(),
        hasher.clone(),
        clock.clone(),
    ));
    let administrative_store = Arc::new(
        PostgresMeasureAdministrativeStore::open(database, hasher.clone(), clock.clone())
            .context("cannot open PostgreSQL administrative measure store")?,
    );
    let administrative = Arc::new(MeasureAdministrativeService::new(
        administrative_store.clone(),
        identity.clone(),
        processor.clone(),
        validator.clone(),
        hasher.clone(),
        clock.clone(),
    ));
    let administrative_reads = Arc::new(MeasureAdministrativeReadService::new(
        administrative_store,
        identity.clone(),
        hasher.clone(),
        clock.clone(),
    ));
    let measure_store = Arc::new(
        PostgresMeasureDecisionStore::open(database, hasher.clone(), clock.clone())
            .context("cannot open PostgreSQL measure decision store")?,
    );
    let decisions = Arc::new(MeasureDecisionRecordService::new(
        measure_store.clone(),
        identity.clone(),
        processor,
        validator,
        hasher.clone(),
        clock.clone(),
    ));
    let decision_reads = Arc::new(MeasureDecisionRecordReadService::new(
        measure_store.clone(),
        identity.clone(),
        hasher.clone(),
        clock.clone(),
    ));
    let records = Arc::new(MeasureRecordReadService::new(
        measure_store,
        identity,
        hasher.clone(),
        clock,
    ));
    Ok(web::PrecautionaryWorkflows {
        context,
        hearings: writes,
        hearing_reads: reads,
        administrative,
        administrative_reads,
        records,
        decisions,
        decision_reads,
        hasher,
    })
}
