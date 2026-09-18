#![allow(dead_code)]
mod values;
pub use crate::case_administration_support::Fixture;
use crate::case_stage_database_support::{FixedClock, TestIdentity};
use application::{deadlines::*, identity::Principal};
use domain::{
    cases::CaseId,
    identity::{Role, UserId},
};
use infrastructure::{PostgresDeadlineStore, RingSha256Hasher};
use std::sync::Arc;
pub use values::*;

pub fn store(db: &Fixture) -> Arc<PostgresDeadlineStore> {
    Arc::new(
        PostgresDeadlineStore::open(
            &db.runtime_url,
            Arc::new(RingSha256Hasher),
            Arc::new(FixedClock(db.at)),
        )
        .unwrap(),
    )
}
pub fn service(db: &Fixture, actor: UserId, role: Role) -> DeadlineService {
    DeadlineService::new(
        store(db),
        Arc::new(TestIdentity(Principal {
            id: actor,
            email: "session@example.test".into(),
            role,
        })),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}
pub fn persist(
    workflow: &dyn DeadlineWorkflow,
    case_id: CaseId,
    command: DeadlineCommand,
) -> DeadlineDetail {
    let draft = workflow
        .prepare("session", case_id, command.clone())
        .unwrap();
    workflow
        .submit("session", case_id, command, draft.submission_digest)
        .unwrap()
}
pub fn prepared(db: &Fixture, actor: UserId, command: &DeadlineCommand) -> PreparedDeadlineChange {
    let preparation = store(db).prepare(actor, db.case, command).unwrap();
    prepare_deadline_change(
        &RingSha256Hasher,
        actor,
        db.case,
        command.clone(),
        preparation,
    )
    .unwrap()
}
pub fn query(limit: u32, after: Option<DeadlineId>) -> DeadlineQuery {
    DeadlineQuery::new(limit, after, DeadlineStatusFilter::All).unwrap()
}
pub fn history_query(limit: u32, before: Option<u32>) -> DeadlineHistoryQuery {
    DeadlineHistoryQuery::new(limit, before).unwrap()
}

pub fn snapshot(db: &mut Fixture) -> serde_json::Value {
    db.admin.query_one("SELECT jsonb_build_object(
        'roots',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM case_deadlines r),
        'revisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY deadline_id,revision) FROM case_deadline_revisions r),
        'events',(SELECT jsonb_agg(to_jsonb(r) ORDER BY sequence) FROM deadline_source_events r),
        'audit',(SELECT jsonb_agg(to_jsonb(r) ORDER BY sequence) FROM audit_events r))", &[]).unwrap().get(0)
}
