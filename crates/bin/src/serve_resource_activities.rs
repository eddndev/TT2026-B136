//! Compose exact resource associations outside the asynchronous runtime.
use anyhow::Context;
use application::{
    identity::IdentityWorkflow,
    resource_activities::{ResourceActivityService, ResourceActivityWorkflow},
};
use infrastructure::{PostgresResourceActivityStore, RingSha256Hasher, SystemClock};
use std::sync::Arc;

pub(crate) fn open(
    database_url: &(impl infrastructure::PostgresConnectionSource + ?Sized),
    identity: Arc<dyn IdentityWorkflow>,
) -> anyhow::Result<Arc<dyn ResourceActivityWorkflow>> {
    let hasher = Arc::new(RingSha256Hasher::new());
    let clock = Arc::new(SystemClock::new());
    let store = PostgresResourceActivityStore::open(database_url, hasher.clone(), clock.clone())
        .context("cannot open PostgreSQL resource activity store")?;
    Ok(Arc::new(ResourceActivityService::new(
        Arc::new(store),
        identity,
        hasher,
        clock,
    )))
}

pub(crate) fn open_deadlines(
    database_url: &(impl infrastructure::PostgresConnectionSource + ?Sized),
    identity: Arc<dyn IdentityWorkflow>,
) -> anyhow::Result<Arc<dyn application::resource_deadlines::ResourceDeadlineWorkflow>> {
    let hasher = Arc::new(RingSha256Hasher::new());
    let clock = Arc::new(SystemClock::new());
    let store = infrastructure::PostgresResourceDeadlineStore::open(
        database_url,
        hasher.clone(),
        clock.clone(),
    )
    .context("cannot open PostgreSQL contextual deadline store")?;
    Ok(Arc::new(
        application::resource_deadlines::ResourceDeadlineService::new(
            Arc::new(store),
            identity,
            hasher,
            clock,
        ),
    ))
}

use application::resource_hearings::{
    ResourceHearingReadService, ResourceHearingReadWorkflow, ResourceHearingService,
    ResourceHearingWorkflow,
};

type HearingWorkflows = (
    Arc<dyn ResourceHearingWorkflow>,
    Arc<dyn ResourceHearingReadWorkflow>,
);

pub(crate) fn open_hearings(
    database_url: &(impl infrastructure::PostgresConnectionSource + ?Sized),
    identity: Arc<dyn IdentityWorkflow>,
) -> anyhow::Result<HearingWorkflows> {
    let hasher = Arc::new(RingSha256Hasher::new());
    let clock = Arc::new(SystemClock::new());
    let store = Arc::new(
        infrastructure::PostgresResourceHearingStore::open(
            database_url,
            hasher.clone(),
            clock.clone(),
        )
        .context("cannot open PostgreSQL resource hearing store")?,
    );
    let writes = Arc::new(ResourceHearingService::new(
        store.clone(),
        identity.clone(),
        hasher.clone(),
        clock.clone(),
    ));
    let reads = Arc::new(ResourceHearingReadService::new(
        store, identity, hasher, clock,
    ));
    Ok((writes, reads))
}
