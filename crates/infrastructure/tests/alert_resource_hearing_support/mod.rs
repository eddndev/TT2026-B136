#![allow(dead_code)]
pub use crate::alert_backend_support::{drive, operation, query, store, MutableClock};
pub(crate) use crate::resource_hearing_database_support as own;
pub use crate::resource_hearing_database_support::Fixture;
use application::{alerts::*, resource_hearings::ResourceHearingCreation, ApplicationError};
use domain::{hearings::HearingTime, resource_hearings::*};
use infrastructure::{PostgresAlertStore, RingSha256Hasher};
use std::sync::Arc;
use time::OffsetDateTime;
use uuid::Uuid;

pub fn create(
    db: &mut Fixture,
    at: OffsetDateTime,
    id: Option<Uuid>,
) -> (
    crate::resource_activity_support::Captures,
    ResourceHearingCreation,
) {
    let (captures, mut command) = own::setup(db);
    if let Some(id) = id {
        command.hearing_id = ResourceHearingId::from_uuid(id);
    }
    let v = &command.values;
    command.values = ResourceHearingValues::new(ResourceHearingValuesInput {
        kind: v.kind(),
        scheduled_at: HearingTime::new(at).unwrap(),
        modality: v.modality(),
        venue: v.venue().clone(),
        note: v.note().cloned(),
        participants: v.participants().to_vec(),
        scheduling_basis: v.scheduling_basis().clone(),
    })
    .unwrap();
    (captures, own::submit(db, command))
}
pub fn subject(created: &ResourceHearingCreation) -> AlertSubject {
    AlertSubject::ResourceHearing {
        case_id: created.origin.case_id,
        resource_id: created.origin.resource_id,
        id: created.origin.hearing_id,
    }
}
pub fn own_alerts(store: &PostgresAlertStore, user: domain::identity::UserId) -> Vec<AlertRecord> {
    store
        .list(user, query(20))
        .unwrap()
        .alerts
        .into_iter()
        .filter(|row| matches!(row.subject, AlertSubject::ResourceHearing { .. }))
        .collect()
}
pub fn open(db: &Fixture) -> Result<PostgresAlertStore, ApplicationError> {
    PostgresAlertStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        Arc::new(MutableClock::new(db.at)),
        None,
    )
}
pub fn data(db: &mut Fixture) -> serde_json::Value {
    db.admin
        .query_one(
            "SELECT jsonb_build_object(
        'state',(SELECT jsonb_agg(to_jsonb(t) ORDER BY kind,id) FROM alert_subject_state t),
        'schedule',(SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM alert_schedule t),
        'notifications',(SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM alert_notifications t),
        'cursor',(SELECT jsonb_agg(to_jsonb(t)) FROM alert_scan_cursor t))",
            &[],
        )
        .unwrap()
        .get(0)
}
pub fn damage(db: &mut Fixture, sql: &str) {
    db.admin
        .batch_execute("SET session_replication_role=replica")
        .unwrap();
    db.admin.batch_execute(sql).unwrap();
    db.admin
        .batch_execute("SET session_replication_role=origin")
        .unwrap();
}
