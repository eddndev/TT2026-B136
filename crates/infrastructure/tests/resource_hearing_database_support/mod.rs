#![allow(dead_code)]
pub use crate::case_administration_support::Fixture;
use crate::case_stage_database_support::{FixedClock, TestIdentity};
use crate::resource_activity_support::{self as links, Captures};
use application::{identity::Principal, resource_activities::*, resource_hearings::*};
use domain::{
    hearings::*,
    identity::{Role, UserId},
    resource_hearings::*,
};
use infrastructure::{PostgresResourceHearingStore, RingSha256Hasher};
use std::sync::Arc;

pub fn store(db: &Fixture) -> Arc<PostgresResourceHearingStore> {
    Arc::new(
        PostgresResourceHearingStore::open(
            &db.runtime_url,
            Arc::new(RingSha256Hasher),
            Arc::new(FixedClock(db.at)),
        )
        .unwrap(),
    )
}
pub fn service(db: &Fixture, actor: UserId, role: Role) -> ResourceHearingService {
    ResourceHearingService::new(
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
pub fn setup(db: &mut Fixture) -> (Captures, ResourceHearingCommand) {
    let captures = Captures::new(db);
    let link = captures.link();
    let ResourceActivityChange::Link { selection } = link.change else {
        unreachable!()
    };
    let support = &captures.resource.sources.supports[0];
    let command = ResourceHearingCommand {
        operation_id: ResourceHearingOperationId::new(),
        hearing_id: ResourceHearingId::new(),
        association_id: link.association_id,
        expected_resource_revision: link.expected_resource_revision,
        resource: selection.resource,
        act: selection.act,
        values: ResourceHearingValues::new(ResourceHearingValuesInput {
            kind: ResourceHearingKind::AppealArguments,
            scheduled_at: HearingTime::new(
                time::OffsetDateTime::parse(
                    "2026-10-12T10:00:00-06:00",
                    &time::format_description::well_known::Rfc3339,
                )
                .unwrap(),
            )
            .unwrap(),
            modality: HearingModality::InPerson,
            venue: HearingVenue::new("Appeal court").unwrap(),
            note: None,
            participants: vec![],
            scheduling_basis: ResourceHearingSchedulingBasis::new(
                HearingNote::new("Declared scheduling order").unwrap(),
                HearingSupportRef::new(support.reference, support.digest),
            ),
        })
        .unwrap(),
    };
    (captures, command)
}
pub fn submit(db: &Fixture, command: ResourceHearingCommand) -> ResourceHearingCreation {
    let workflow = service(db, db.owner, Role::Owner);
    let draft = workflow
        .prepare("session", db.case, command.resource.id, command.clone())
        .unwrap();
    workflow
        .submit(
            "session",
            db.case,
            command.resource.id,
            command,
            draft.submission_digest,
        )
        .unwrap()
}
pub fn atomic_rows(db: &mut Fixture) -> serde_json::Value {
    let audit: serde_json::Value = db.admin.query_one(
        "SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY sequence),'[]') FROM audit_events a WHERE action NOT IN ('resource_hearing.prepare','resource_hearing.replay')", &[]).unwrap().get(0);
    serde_json::json!({"hearings":links::rows(db,"case_resource_hearings"),
        "revisions":links::rows(db,"case_resource_hearing_revisions"),
        "associations":links::associations(db),"audit":audit})
}
