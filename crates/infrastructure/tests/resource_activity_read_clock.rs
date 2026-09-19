mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;
mod procedural_fact_backend_support;
mod procedural_resource_support;
mod resource_activity_support;
use application::resource_activities::*;
use domain::{
    clock::{Clock, OffsetDateTime},
    identity::Role,
};
use infrastructure::{PostgresResourceActivityStore, RingSha256Hasher};
use resource_activity_support::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

struct LockedClock {
    url: String,
    at: OffsetDateTime,
    calls: AtomicUsize,
}
impl Clock for LockedClock {
    fn now(&self) -> OffsetDateTime {
        let obtained: bool = postgres::Client::connect(&self.url, postgres::NoTls)
            .unwrap()
            .query_one("SELECT pg_try_advisory_xact_lock(280603412820)", &[])
            .unwrap()
            .get(0);
        assert!(
            !obtained,
            "read clock was captured before the shared audit lock"
        );
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.at
    }
}

#[test]
fn reads_use_one_post_lock_observation_after_a_writer_committed_since_request_start() {
    let Some(mut db) = Fixture::new() else { return };
    let captures = Captures::new(&mut db);
    let workflow = service(&db, db.owner, Role::Owner);
    let linked = persist(&workflow, db.case, captures.resource.id, captures.link());
    persist(&workflow, db.case, captures.resource.id, captures.link());
    let request_started = db.at - time::Duration::seconds(1);
    let observed = db.at + time::Duration::seconds(1);
    let clock = Arc::new(LockedClock {
        url: db.admin_url.clone(),
        at: observed,
        calls: AtomicUsize::new(0),
    });
    let adapter = PostgresResourceActivityStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        clock.clone(),
    )
    .unwrap();
    let view = adapter
        .get(
            db.owner,
            db.case,
            captures.resource.id,
            linked.id,
            None,
            request_started,
        )
        .unwrap();
    assert_eq!(view.association, linked);
    assert_eq!(view.checked_at, observed);
    let page = adapter
        .list(
            db.owner,
            db.case,
            captures.resource.id,
            query(None, None),
            request_started,
        )
        .unwrap();
    assert_eq!(page.associations.len(), 2);
    assert!(page
        .associations
        .iter()
        .all(|value| value.checked_at == observed));
    assert_eq!(clock.calls.load(Ordering::SeqCst), 2);
}
