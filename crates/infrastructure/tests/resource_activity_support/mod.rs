#![allow(dead_code)]
pub mod intercept;

pub use crate::case_administration_support::Fixture;
use crate::case_stage_database_support::{FixedClock, TestIdentity};
use crate::{hearing_database_support as hearings, procedural_resource_support as resources};
use application::{
    hearings::HearingDetail, identity::Principal, procedural_resources::*, resource_activities::*,
};
use domain::{
    cases::CaseId,
    identity::{Role, UserId},
};
use infrastructure::{PostgresResourceActivityStore, RingSha256Hasher};
use std::sync::Arc;

pub fn store(db: &Fixture) -> Arc<PostgresResourceActivityStore> {
    Arc::new(
        PostgresResourceActivityStore::open(
            &db.runtime_url,
            Arc::new(RingSha256Hasher),
            Arc::new(FixedClock(db.at)),
        )
        .unwrap(),
    )
}
pub fn service(db: &Fixture, actor: UserId, role: Role) -> ResourceActivityService {
    service_with_store(db, actor, role, store(db))
}
pub fn service_with_store(
    db: &Fixture,
    actor: UserId,
    role: Role,
    store: Arc<dyn ResourceActivityStore>,
) -> ResourceActivityService {
    ResourceActivityService::new(
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
pub fn persist(
    workflow: &ResourceActivityService,
    case: CaseId,
    resource: ResourceId,
    command: ResourceActivityCommand,
) -> ResourceActivityDetail {
    let draft = workflow
        .prepare("session", case, resource, command.clone())
        .unwrap();
    workflow
        .submit("session", case, resource, command, draft.submission_digest)
        .unwrap()
}
pub struct Captures {
    pub resource: ResourceDetail,
    pub act: ResourceDetail,
    pub head: ResourceDetail,
    pub hearing: HearingDetail,
}
impl Captures {
    pub fn new(db: &mut Fixture) -> Self {
        hearings::complete(db);
        let workflow = resources::service(db, db.owner, Role::Owner);
        let resource = resources::persist(&workflow, db.case, resources::registration(db));
        let act = resources::persist(&workflow, db.case, resources::act(&resource));
        let old = act.act.as_ref().unwrap();
        let head = resources::persist(
            &workflow,
            db.case,
            ResourceCommand {
                operation_id: ResourceOperationId::new(),
                resource_id: resource.id,
                change: ResourceChange::CorrectAct {
                    expected_revision: act.revision,
                    act_id: old.id,
                    expected_act_revision: old.revision,
                    values: old.values.clone(),
                    reason: resources::text("Correct declared act"),
                },
            },
        );
        let hearing = hearings::persist(
            &hearings::service(db, db.owner, Role::Owner),
            db.case,
            hearings::schedule(),
        );
        Self {
            resource,
            act,
            head,
            hearing,
        }
    }
    pub fn link(&self) -> ResourceActivityCommand {
        let act = self.act.act.as_ref().unwrap();
        ResourceActivityCommand {
            operation_id: ResourceActivityOperationId::new(),
            association_id: ResourceActivityId::new(),
            expected_resource_revision: self.head.revision,
            change: ResourceActivityChange::Link {
                selection: ResourceActivitySelection {
                    resource: ResourceCaptureRef {
                        id: self.resource.id,
                        revision: self.resource.revision,
                        capture_digest: self.resource.receipt.capture_digest,
                    },
                    act: Some(ResourceActCaptureRef {
                        id: act.id,
                        revision: act.revision,
                        resource_revision: self.act.revision,
                        capture_digest: self.act.receipt.capture_digest,
                    }),
                    target: ResourceActivityTarget::Hearing {
                        id: self.hearing.snapshot.id,
                        revision: self.hearing.snapshot.revision,
                        submission_digest: self.hearing.snapshot.receipt.submission_digest,
                    },
                },
            },
        }
    }
    pub fn archive(&self, db: &Fixture) -> ResourceDetail {
        resources::persist(
            &resources::service(db, db.owner, Role::Owner),
            db.case,
            ResourceCommand {
                operation_id: ResourceOperationId::new(),
                resource_id: self.resource.id,
                change: ResourceChange::Archive {
                    expected_revision: self.head.revision,
                    reason: resources::text("Archive organization"),
                },
            },
        )
    }
}
pub fn unlink(base: &ResourceActivityDetail, head: ResourceRevision) -> ResourceActivityCommand {
    ResourceActivityCommand {
        operation_id: ResourceActivityOperationId::new(),
        association_id: base.id,
        expected_resource_revision: head,
        change: ResourceActivityChange::Unlink {
            expected_revision: base.revision,
            reason: resources::text("Remove organizational link"),
        },
    }
}
pub fn query(
    kind: Option<ResourceActivityKind>,
    status: Option<ResourceActivityStatus>,
) -> ResourceActivityQuery {
    ResourceActivityQuery::new(20, None, kind, status).unwrap()
}
pub fn history_query() -> ResourceActivityHistoryQuery {
    ResourceActivityHistoryQuery::new(20, None).unwrap()
}
pub fn rows(db: &mut Fixture, table: &str) -> serde_json::Value {
    db.admin
        .query_one(
            &format!(
        "SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text),'[]') FROM {table} t"
    ),
            &[],
        )
        .unwrap()
        .get(0)
}
pub fn associations(db: &mut Fixture) -> serde_json::Value {
    serde_json::json!({
        "roots":rows(db,"case_resource_activity_associations"),
        "revisions":rows(db,"case_resource_activity_association_revisions"),
    })
}
pub fn business_and_audit(db: &mut Fixture) -> serde_json::Value {
    let audit: serde_json::Value = db.admin.query_one(
        "SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY sequence),'[]') FROM audit_events a WHERE action<>'resource_activity.prepare'", &[],
    ).unwrap().get(0);
    serde_json::json!({"associations":associations(db),"audit":audit})
}
pub fn independent_rows(db: &mut Fixture) -> serde_json::Value {
    const TABLES: &[&str] = &[
        "cases",
        "case_administration_revisions",
        "case_stage_revisions",
        "case_procedural_resources",
        "case_procedural_resource_revisions",
        "case_procedural_resource_acts",
        "case_hearings",
        "case_hearing_revisions",
        "case_deadlines",
        "case_deadline_revisions",
        "deadline_source_events",
        "deadline_dispatch_cursor",
        "deadline_reevaluation_jobs",
        "deadline_reevaluation_results",
        "deadline_reevaluation_attempts",
        "alert_preferences",
        "alert_subject_state",
        "alert_scan_cursor",
        "alert_schedule",
        "alert_notifications",
        "alert_read_receipts",
        "alert_email_outbox",
        "alert_email_attempts",
    ];
    let mut values = serde_json::Map::new();
    for table in TABLES {
        values.insert((*table).into(), rows(db, table));
    }
    serde_json::Value::Object(values)
}
