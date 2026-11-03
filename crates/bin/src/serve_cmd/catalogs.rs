//! Compose the global judicial calendar and deadline profile catalogs.
use anyhow::Context;
use application::{
    deadline_profiles::DeadlineProfileService, identity::IdentityWorkflow,
    judicial_calendars::JudicialCalendarService,
};
use infrastructure::{
    PostgresDeadlineProfileStore, PostgresJudicialCalendarStore, RingSha256Hasher, SystemClock,
};
use std::sync::Arc;

pub(super) fn open(
    database: &(impl infrastructure::PostgresConnectionSource + ?Sized),
    identity: Arc<dyn IdentityWorkflow>,
) -> anyhow::Result<(JudicialCalendarService, DeadlineProfileService)> {
    let calendar_hasher = Arc::new(RingSha256Hasher::new());
    let calendar_clock = Arc::new(SystemClock::new());
    let calendars = JudicialCalendarService::new(
        Arc::new(
            PostgresJudicialCalendarStore::open(
                database,
                calendar_hasher.clone(),
                calendar_clock.clone(),
            )
            .context("cannot open PostgreSQL judicial calendar store")?,
        ),
        identity.clone(),
        calendar_hasher,
        calendar_clock,
    );
    let profile_hasher = Arc::new(RingSha256Hasher::new());
    let profile_clock = Arc::new(SystemClock::new());
    let profiles = DeadlineProfileService::new(
        Arc::new(
            PostgresDeadlineProfileStore::open(
                database,
                profile_hasher.clone(),
                profile_clock.clone(),
            )
            .context("cannot open PostgreSQL deadline profile store")?,
        ),
        identity,
        profile_hasher,
        profile_clock,
    );
    Ok((calendars, profiles))
}
