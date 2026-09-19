#![allow(dead_code)]
mod values;
pub use crate::case_administration_support::Fixture;
use crate::case_stage_database_support::{FixedClock, TestIdentity};
pub use application::deadline_tracking::{TrackingPolicies, TrackingPolicy};
use application::{deadlines::*, identity::Principal};
use domain::{
    cases::CaseId,
    identity::{Role, UserId},
};
use infrastructure::{PostgresDeadlineStore, RingSha256Hasher};
use std::sync::Arc;
pub use values::*;

pub const FOLLOW_RESOLUTION: TrackingPolicies = TrackingPolicies {
    profile: TrackingPolicy::Follow,
    source: TrackingPolicy::Follow,
    calendar: TrackingPolicy::Undetermined,
};

pub fn human(command: DeadlineCommand, policies: Option<TrackingPolicies>) -> DeadlineHumanCommand {
    DeadlineHumanCommand::new(command, policies).unwrap()
}

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
pub fn persist(
    workflow: &dyn DeadlineWorkflow,
    case_id: CaseId,
    command: DeadlineHumanCommand,
) -> DeadlineDetail {
    let draft = workflow
        .prepare("session", case_id, command.clone())
        .unwrap();
    workflow
        .submit("session", case_id, command, draft.submission_digest)
        .unwrap()
}
/// Prepare historical V1 fixture evidence without using the human workflow.
pub fn prepared_legacy(
    db: &Fixture,
    actor: UserId,
    command: &DeadlineCommand,
) -> PreparedDeadlineChange {
    legacy_preparation(store(db).as_ref(), db, actor, command)
}

/// Seed V1 history for migration, reconstruction and durable legacy reconciliation.
pub fn persist_legacy(db: &Fixture, actor: UserId, command: DeadlineCommand) -> DeadlineDetail {
    let repository = store(db);
    let prepared = legacy_preparation(repository.as_ref(), db, actor, &command);
    repository.commit(actor, prepared).unwrap()
}

fn legacy_preparation(
    repository: &dyn DeadlineStore,
    db: &Fixture,
    actor: UserId,
    command: &DeadlineCommand,
) -> PreparedDeadlineChange {
    let preparation = repository.prepare(actor, db.case, command).unwrap();
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
