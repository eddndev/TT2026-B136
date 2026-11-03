//! Compose the deadline dispatcher, worker and the read and write services that
//! share one deadline hasher and clock.
use anyhow::Context;
use application::{
    agenda::AgendaService, dashboard::DashboardService, deadlines::DeadlineService,
    identity::IdentityWorkflow,
};
use infrastructure::{
    PostgresAgendaStore, PostgresDashboardStore, PostgresDeadlineDispatchStore,
    PostgresDeadlineStore, PostgresDeadlineWorkerStore, RingSha256Hasher, SystemClock,
};
use std::sync::Arc;

pub(super) struct DeadlineServices {
    pub(super) dispatch: PostgresDeadlineDispatchStore,
    pub(super) worker: PostgresDeadlineWorkerStore,
    pub(super) dashboard: DashboardService,
    pub(super) agenda: AgendaService,
    pub(super) deadlines: DeadlineService,
}

pub(super) fn open(
    database: &(impl infrastructure::PostgresConnectionSource + ?Sized),
    identity: Arc<dyn IdentityWorkflow>,
) -> anyhow::Result<DeadlineServices> {
    let deadline_hasher = Arc::new(RingSha256Hasher::new());
    let deadline_clock = Arc::new(SystemClock::new());
    let dispatch = PostgresDeadlineDispatchStore::open(
        database,
        deadline_hasher.clone(),
        deadline_clock.clone(),
    )
    .context("cannot open PostgreSQL deadline dispatcher")?;
    let worker = PostgresDeadlineWorkerStore::open(
        database,
        deadline_hasher.clone(),
        deadline_clock.clone(),
    )
    .context("cannot open PostgreSQL deadline worker")?;
    let dashboard = DashboardService::new(
        Arc::new(
            PostgresDashboardStore::open(database, deadline_hasher.clone(), deadline_clock.clone())
                .context("cannot open PostgreSQL dashboard store")?,
        ),
        identity.clone(),
    );
    let agenda = AgendaService::new(
        Arc::new(
            PostgresAgendaStore::open(database, deadline_hasher.clone(), deadline_clock.clone())
                .context("cannot open PostgreSQL agenda store")?,
        ),
        identity.clone(),
    );
    let deadlines = DeadlineService::new(
        Arc::new(
            PostgresDeadlineStore::open(database, deadline_hasher.clone(), deadline_clock.clone())
                .context("cannot open PostgreSQL deadline store")?,
        ),
        identity,
        deadline_hasher,
        deadline_clock,
    );
    Ok(DeadlineServices {
        dispatch,
        worker,
        dashboard,
        agenda,
        deadlines,
    })
}
