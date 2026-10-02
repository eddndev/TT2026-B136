#![allow(dead_code)]

pub use crate::case_administration_support::Fixture;
pub use application::audit_query::*;
pub use application::identity::Principal;
pub use application::ApplicationError;
pub use domain::audit::{AuditLog, ChainVerification};
pub use domain::identity::Role;
pub use infrastructure::{PostgresAuditEventStore, PostgresAuditLog, RingSha256Hasher};
pub use time::{Duration, OffsetDateTime};

pub fn owner(db: &Fixture) -> Principal {
    Principal {
        id: db.owner,
        email: "owner@example.test".into(),
        role: Role::Owner,
    }
}
pub fn query(db: &Fixture, limit: u32, cursor: Option<&str>) -> AuditEventQuery {
    AuditEventQuery::new(
        db.at - Duration::days(1),
        db.at + Duration::days(1),
        None,
        None,
        None,
        limit,
        cursor,
    )
    .unwrap()
}
pub fn store(db: &Fixture) -> PostgresAuditEventStore {
    PostgresAuditEventStore::open(&db.runtime_url).unwrap()
}
pub fn append(db: &Fixture, actor: &str, action: &str, resource: &str, at: OffsetDateTime) -> u64 {
    PostgresAuditLog::open(&db.runtime_url)
        .unwrap()
        .append(actor, action, resource, at)
        .unwrap()
        .event
        .sequence
}
pub fn count(db: &mut Fixture) -> i64 {
    db.admin
        .query_one("SELECT count(*) FROM audit_events", &[])
        .unwrap()
        .get(0)
}
pub fn trail(db: &Fixture) -> Vec<domain::audit::ChainedEvent> {
    PostgresAuditLog::open(&db.runtime_url)
        .unwrap()
        .load_all()
        .unwrap()
}
pub fn assert_chain(db: &Fixture) {
    let events = trail(db);
    assert_eq!(
        domain::audit::verify_chain(&RingSha256Hasher, &events).unwrap(),
        ChainVerification::Valid {
            entries: events.len()
        }
    );
}
pub fn original_columns(db: &mut Fixture) -> serde_json::Value {
    db.admin.query_one("SELECT coalesce(jsonb_agg(jsonb_build_array(sequence,timestamp,actor,action,resource,encode(chain,'hex')) ORDER BY sequence),'[]'::jsonb) FROM audit_events", &[]).unwrap().get(0)
}
