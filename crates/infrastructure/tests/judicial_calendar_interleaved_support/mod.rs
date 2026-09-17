#![allow(dead_code)]
use super::{
    case_stage_database_support::{FixedClock, TestIdentity},
    judicial_calendar_database_support::{store, Fixture},
};
use application::{identity::Principal, judicial_calendars::*, ApplicationError};
use domain::{
    clock::OffsetDateTime,
    identity::{Role, UserId},
};
use std::sync::Arc;
struct InterleavedStore {
    inner: Arc<dyn JudicialCalendarStore>,
    after_prepare: Box<dyn Fn() + Send + Sync>,
}
impl JudicialCalendarStore for InterleavedStore {
    fn prepare(
        &self,
        actor: UserId,
        command: &JudicialCalendarCommand,
    ) -> Result<JudicialCalendarPreparation, ApplicationError> {
        let result = self.inner.prepare(actor, command)?;
        (self.after_prepare)();
        Ok(result)
    }
    fn commit(
        &self,
        actor: UserId,
        prepared: PreparedJudicialCalendarChange,
    ) -> Result<JudicialCalendarDetail, ApplicationError> {
        self.inner.commit(actor, prepared)
    }
    fn list(
        &self,
        actor: UserId,
        query: JudicialCalendarQuery,
        at: OffsetDateTime,
    ) -> Result<JudicialCalendarPage, ApplicationError> {
        self.inner.list(actor, query, at)
    }
    fn get(
        &self,
        actor: UserId,
        id: JudicialCalendarId,
        revision: Option<JudicialCalendarRevision>,
        at: OffsetDateTime,
    ) -> Result<JudicialCalendarDetail, ApplicationError> {
        self.inner.get(actor, id, revision, at)
    }
    fn history(
        &self,
        actor: UserId,
        id: JudicialCalendarId,
        query: JudicialCalendarHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<JudicialCalendarHistoryPage, ApplicationError> {
        self.inner.history(actor, id, query, at)
    }
}
pub fn interleaved(
    inner: Arc<dyn JudicialCalendarStore>,
    change: impl Fn() + Send + Sync + 'static,
) -> Arc<dyn JudicialCalendarStore> {
    Arc::new(InterleavedStore {
        inner,
        after_prepare: Box::new(change),
    })
}
pub fn hooked(
    db: &Fixture,
    actor: UserId,
    change: impl Fn() + Send + Sync + 'static,
) -> JudicialCalendarService {
    JudicialCalendarService::new(
        interleaved(store(db), change),
        Arc::new(TestIdentity(Principal {
            id: actor,
            email: "session@example.test".into(),
            role: Role::Owner,
        })),
        Arc::new(infrastructure::RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}
pub fn wait_for_lock(db: &mut Fixture) -> bool {
    let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while std::time::Instant::now() < until {
        if db.admin.query_one("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE usename=$1 AND wait_event='advisory')",&[&db.role]).unwrap().get::<_,bool>(0) {return true;}
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    false
}
