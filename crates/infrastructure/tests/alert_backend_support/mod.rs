#![allow(dead_code)]

pub mod atomicity;

use crate::{deadline_backend_support as deadlines, hearing_database_support as hearings};
use application::{alerts::*, deadlines::*, hearings::*, procedural_facts::FactDetail};
use domain::{
    clock::{Clock, OffsetDateTime},
    identity::{Role, UserId},
};
use infrastructure::{PostgresAlertStore, RingSha256Hasher};
use std::sync::{Arc, Mutex};
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

pub use crate::case_administration_support::Fixture;

pub struct MutableClock(Mutex<OffsetDateTime>);

impl MutableClock {
    pub fn new(at: OffsetDateTime) -> Self {
        Self(Mutex::new(at))
    }

    pub fn set(&self, at: OffsetDateTime) {
        *self.0.lock().unwrap() = at;
    }
}

impl Clock for MutableClock {
    fn now(&self) -> OffsetDateTime {
        *self.0.lock().unwrap()
    }
}

pub fn fixture() -> Option<Fixture> {
    let mut db = Fixture::new()?;
    hearings::complete(&mut db);
    Some(db)
}

pub fn store(db: &Fixture, clock: Arc<MutableClock>) -> PostgresAlertStore {
    PostgresAlertStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        clock,
        Some(AlertEmailConfiguration {
            from_email: "alerts@example.test".into(),
            login_url: "https://example.test/login".into(),
        }),
    )
    .unwrap()
}

pub fn query(limit: u32) -> AlertQuery {
    AlertQuery::new(limit, AlertReadFilter::All, AlertStateFilter::All, None).unwrap()
}

pub fn operation() -> AlertOperationId {
    AlertOperationId::from_uuid(Uuid::new_v4())
}

pub fn drive(store: &PostgresAlertStore) {
    for _ in 0..16 {
        if matches!(store.run_next().unwrap(), AlertSchedulerRun::Idle) {
            return;
        }
    }
}

pub fn hearing(db: &Fixture, at: OffsetDateTime) -> HearingDetail {
    let mut command = hearings::schedule();
    command.hearing_id = HearingId::from_uuid(Uuid::new_v4());
    let HearingChange::Schedule { values, .. } = &mut command.change else {
        unreachable!()
    };
    *values = hearings::values(&at.format(&Rfc3339).unwrap());
    hearings::persist(
        &hearings::service(db, db.owner, Role::Owner),
        db.case,
        command,
    )
}

pub fn accepted_deadline(db: &Fixture, responsible: UserId) -> (FactDetail, DeadlineDetail) {
    let profile = deadlines::profile(db);
    let source = deadlines::source(db);
    let mut command = deadlines::command(db, &profile, &source);
    deadlines::definition_mut(&mut command).responsible = responsible;
    let row = deadlines::persist(
        &deadlines::service(db, db.owner, Role::Owner),
        db.case,
        deadlines::human(command, Some(deadlines::FOLLOW_RESOLUTION)),
    );
    assert!(row.calculation.result.due_at().is_some());
    (source, row)
}

pub fn resources(db: &mut Fixture) -> serde_json::Value {
    db.admin
        .query_one(
            "SELECT jsonb_build_object(
        'hearings',(SELECT jsonb_agg(to_jsonb(r) ORDER BY hearing_id,revision)
            FROM case_hearing_revisions r),
        'deadlines',(SELECT jsonb_agg(to_jsonb(r) ORDER BY deadline_id,revision)
            FROM case_deadline_revisions r),
        'events',(SELECT jsonb_agg(to_jsonb(r) ORDER BY sequence)
            FROM deadline_source_events r))",
            &[],
        )
        .unwrap()
        .get(0)
}

pub fn corrupt_hearing_context(db: &mut Fixture, id: HearingId) {
    db.admin
        .batch_execute("SET session_replication_role='replica'")
        .unwrap();
    db.admin
        .execute(
            "UPDATE case_hearing_revisions
        SET scheduling_administration_digest=decode(repeat('00',32),'hex'),
            recorded_administration_digest=decode(repeat('00',32),'hex')
        WHERE hearing_id=$1",
            &[&id.as_uuid()],
        )
        .unwrap();
    db.admin
        .batch_execute("SET session_replication_role='origin'")
        .unwrap();
}
