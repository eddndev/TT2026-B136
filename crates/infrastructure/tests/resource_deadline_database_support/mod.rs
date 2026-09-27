#![allow(dead_code)]
mod boundaries;
pub use crate::case_administration_support::Fixture;
use crate::case_stage_database_support::{FixedClock, TestIdentity};
use crate::{
    deadline_backend_support as deadlines,
    resource_activity_support::{self as links, Captures},
};
use application::{identity::Principal, resource_activities::*, resource_deadlines::*};
use domain::identity::{Role, UserId};
use infrastructure::{PostgresResourceDeadlineStore, RingSha256Hasher};
use std::sync::Arc;
pub fn store(db: &Fixture) -> Arc<PostgresResourceDeadlineStore> {
    Arc::new(
        PostgresResourceDeadlineStore::open(
            &db.runtime_url,
            Arc::new(RingSha256Hasher),
            Arc::new(FixedClock(db.at)),
        )
        .unwrap(),
    )
}
pub fn service(db: &Fixture, actor: UserId, role: Role) -> ResourceDeadlineService {
    service_with_store(db, actor, role, store(db))
}
pub fn service_with_store(
    db: &Fixture,
    actor: UserId,
    role: Role,
    store: Arc<dyn ResourceDeadlineStore>,
) -> ResourceDeadlineService {
    ResourceDeadlineService::new(
        store,
        Arc::new(TestIdentity(Principal {
            id: actor,
            email: if actor == db.owner {
                "owner@example.test".into()
            } else {
                format!("{actor}@example.test")
            },
            role,
        })),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}
pub fn setup(db: &mut Fixture) -> (Captures, ResourceDeadlineCommand) {
    let captures = Captures::new(db);
    let link = captures.link();
    let ResourceActivityChange::Link { selection } = link.change else {
        unreachable!()
    };
    let command = ResourceDeadlineCommand {
        association_id: link.association_id,
        expected_resource_revision: link.expected_resource_revision,
        resource: selection.resource,
        act: selection.act,
        deadline: deadlines::human(deadlines::setup(db), Some(deadlines::FOLLOW_RESOLUTION)),
    };
    (captures, command)
}
pub fn pair_rows(db: &mut Fixture) -> serde_json::Value {
    serde_json::json!({"deadlines":links::rows(db,"case_deadline_revisions"),"associations":links::rows(db,"case_resource_activity_association_revisions")})
}
pub fn atomic_rows(db: &mut Fixture) -> serde_json::Value {
    let audits:serde_json::Value=db.admin.query_one("SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY sequence),'[]') FROM audit_events a WHERE action<>'resource_deadline.prepare'",&[]).unwrap().get(0);
    serde_json::json!({"pair":pair_rows(db),"deadline_roots":links::rows(db,"case_deadlines"),"association_roots":links::rows(db,"case_resource_activity_associations"),"events":links::rows(db,"deadline_source_events"),"audit":audits})
}
