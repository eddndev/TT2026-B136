//! Compose atomic hearing consequences using the same identity and document admission.
use anyhow::Context;
use application::{
    documents::{DocumentFormatBatchValidator, DocumentProcessor},
    hearing_derived_deadlines::{HearingDerivedDeadlineService, HearingDerivedDeadlineWorkflow},
    identity::IdentityWorkflow,
};
use infrastructure::{PostgresHearingDerivedDeadlineStore, RingSha256Hasher, SystemClock};
use std::sync::Arc;

pub(super) fn open(
    database: &(impl infrastructure::PostgresConnectionSource + ?Sized),
    identity: Arc<dyn IdentityWorkflow>,
    processor: Arc<DocumentProcessor>,
    validator: Arc<dyn DocumentFormatBatchValidator>,
) -> anyhow::Result<Arc<dyn HearingDerivedDeadlineWorkflow>> {
    let hasher = Arc::new(RingSha256Hasher::new());
    let clock = Arc::new(SystemClock::new());
    let store = PostgresHearingDerivedDeadlineStore::open(database, hasher.clone(), clock.clone())
        .context("cannot open PostgreSQL hearing derived deadline store")?;
    Ok(Arc::new(HearingDerivedDeadlineService::new(
        Arc::new(store),
        identity,
        processor,
        validator,
        hasher,
        clock,
    )))
}
