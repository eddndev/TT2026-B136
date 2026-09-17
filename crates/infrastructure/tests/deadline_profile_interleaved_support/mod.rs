#![allow(dead_code)]

mod case_actions;

use crate::{
    case_stage_database_support::{FixedClock, TestIdentity},
    deadline_profile_database_support::{store, Fixture},
};
use application::{deadline_profiles::*, identity::Principal, ApplicationError};
use domain::{
    clock::OffsetDateTime,
    identity::{Role, UserId},
};
use postgres::{Client, NoTls};
use std::sync::{Arc, Mutex};

struct InterleavedStore {
    inner: Arc<dyn DeadlineProfileStore>,
    after_prepare: Box<dyn Fn() + Send + Sync>,
}
impl DeadlineProfileStore for InterleavedStore {
    fn prepare(
        &self,
        actor: UserId,
        collection: DeadlineProfileCollection,
        command: &DeadlineProfileCommand,
    ) -> Result<DeadlineProfilePreparation, ApplicationError> {
        let prepared = self.inner.prepare(actor, collection, command)?;
        (self.after_prepare)();
        Ok(prepared)
    }
    fn commit(
        &self,
        actor: UserId,
        prepared: PreparedDeadlineProfileChange,
    ) -> Result<DeadlineProfileDetail, ApplicationError> {
        self.inner.commit(actor, prepared)
    }
    fn list(
        &self,
        actor: UserId,
        collection: DeadlineProfileCollection,
        query: DeadlineProfileQuery,
        at: OffsetDateTime,
    ) -> Result<DeadlineProfilePage, ApplicationError> {
        self.inner.list(actor, collection, query, at)
    }
    fn get(
        &self,
        actor: UserId,
        collection: DeadlineProfileCollection,
        id: DeadlineProfileId,
        revision: Option<DeadlineProfileRevision>,
        at: OffsetDateTime,
    ) -> Result<DeadlineProfileDetail, ApplicationError> {
        self.inner.get(actor, collection, id, revision, at)
    }
    fn history(
        &self,
        actor: UserId,
        collection: DeadlineProfileCollection,
        id: DeadlineProfileId,
        query: DeadlineProfileHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<DeadlineProfileHistoryPage, ApplicationError> {
        self.inner.history(actor, collection, id, query, at)
    }
}
pub fn hooked(
    db: &Fixture,
    actor: UserId,
    change: impl Fn() + Send + Sync + 'static,
) -> DeadlineProfileService {
    DeadlineProfileService::new(
        Arc::new(InterleavedStore {
            inner: store(db),
            after_prepare: Box::new(change),
        }),
        Arc::new(TestIdentity(Principal {
            id: actor,
            email: "session@example.test".into(),
            role: Role::Owner,
        })),
        Arc::new(infrastructure::RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}
fn rows(client: &mut Client) -> serde_json::Value {
    client.query_one("SELECT jsonb_build_object(
        'roots',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM deadline_profiles r),
        'revisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY profile_id,revision) FROM deadline_profile_revisions r),
        'events',(SELECT jsonb_agg(to_jsonb(r) ORDER BY sequence) FROM deadline_source_events r),
        'audit',(SELECT jsonb_agg(to_jsonb(r) ORDER BY sequence) FROM audit_events r))", &[]).unwrap().get(0)
}
pub type Captured = Arc<Mutex<Option<serde_json::Value>>>;
pub fn watched(
    db: &Fixture,
    actor: UserId,
    change: impl Fn(&mut Client) + Send + Sync + 'static,
) -> (DeadlineProfileService, Captured) {
    let captured = Arc::new(Mutex::new(None));
    let copy = captured.clone();
    let url = db.admin_url.clone();
    let workflow = hooked(db, actor, move || {
        let mut client = Client::connect(&url, NoTls).unwrap();
        change(&mut client);
        *copy.lock().unwrap() = Some(rows(&mut client));
    });
    (workflow, captured)
}
pub fn unchanged(db: &mut Fixture, captured: Captured) {
    assert_eq!(
        rows(&mut db.admin),
        captured
            .lock()
            .unwrap()
            .clone()
            .expect("prepare hook was not reached")
    );
}
pub fn counts(db: &mut Fixture) -> (i64, i64, i64, i64) {
    let row = db
        .admin
        .query_one(
            "SELECT
        (SELECT count(*) FROM deadline_profiles),
        (SELECT count(*) FROM deadline_profile_revisions),
        (SELECT count(*) FROM deadline_source_events),
        (SELECT count(*) FROM audit_events)",
            &[],
        )
        .unwrap();
    (row.get(0), row.get(1), row.get(2), row.get(3))
}
