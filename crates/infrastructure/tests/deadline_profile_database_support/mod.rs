#![allow(dead_code)]
pub use crate::case_administration_support::Fixture;
use crate::case_stage_database_support::{FixedClock, TestIdentity};
#[path = "../../../application/tests/deadline_profile_catalog_support/values.rs"]
mod values;
use application::{deadline_profiles::*, identity::Principal};
use domain::{
    cases::CaseId,
    identity::{Role, UserId},
};
use infrastructure::{PostgresDeadlineProfileStore, RingSha256Hasher};
use std::sync::Arc;
pub use values::*;
pub fn store(db: &Fixture) -> Arc<PostgresDeadlineProfileStore> {
    Arc::new(
        PostgresDeadlineProfileStore::open(
            &db.runtime_url,
            Arc::new(RingSha256Hasher),
            Arc::new(FixedClock(db.at)),
        )
        .unwrap(),
    )
}
pub fn service(db: &Fixture, actor: UserId, role: Role) -> DeadlineProfileService {
    DeadlineProfileService::new(
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
pub fn publish(case: Option<CaseId>) -> DeadlineProfileCommand {
    DeadlineProfileCommand {
        operation_id: DeadlineProfileOperationId::new(),
        profile_id: DeadlineProfileId::new(),
        change: DeadlineProfileChange::Publish {
            definition: definition(case),
        },
    }
}
pub fn replace(base: &DeadlineProfileDetail) -> DeadlineProfileCommand {
    DeadlineProfileCommand {
        operation_id: DeadlineProfileOperationId::new(),
        profile_id: base.id,
        change: DeadlineProfileChange::Replace {
            expected_revision: base.revision,
            definition: base.definition.clone(),
            reason: text("Rechecked source"),
        },
    }
}
pub fn retire(base: &DeadlineProfileDetail) -> DeadlineProfileCommand {
    DeadlineProfileCommand {
        operation_id: DeadlineProfileOperationId::new(),
        profile_id: base.id,
        change: DeadlineProfileChange::Retire {
            expected_revision: base.revision,
            reason: text("Retired configuration"),
        },
    }
}
pub fn persist(
    workflow: &DeadlineProfileService,
    collection: DeadlineProfileCollection,
    command: DeadlineProfileCommand,
) -> DeadlineProfileDetail {
    let draft = workflow
        .prepare("session", collection, command.clone())
        .unwrap();
    workflow
        .submit("session", collection, command, draft.submission_digest)
        .unwrap()
}
pub fn query(limit: u32, after: Option<DeadlineProfileId>) -> DeadlineProfileQuery {
    DeadlineProfileQuery::new(limit, after, DeadlineProfileStatusFilter::All).unwrap()
}
pub fn snapshot(db: &mut Fixture) -> serde_json::Value {
    db.admin.query_one("SELECT jsonb_build_object(
        'roots',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM deadline_profiles r),
        'revisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY profile_id,revision) FROM deadline_profile_revisions r),
        'events',(SELECT jsonb_agg(to_jsonb(r) ORDER BY sequence) FROM deadline_source_events r),
        'audit',(SELECT jsonb_agg(to_jsonb(r) ORDER BY sequence) FROM audit_events r))",&[]).unwrap().get(0)
}
