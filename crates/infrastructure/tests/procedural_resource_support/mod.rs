#![allow(dead_code)]

pub use super::case_administration_support::Fixture;
use super::case_stage_database_support::{processor, FixedClock, FormatCheck, TestIdentity};
use super::procedural_fact_backend_support as facts;
use application::{identity::Principal, procedural_facts::*, procedural_resources::*};
use domain::{
    cases::CaseId,
    identity::{Role, UserId},
    procedural_time::DeclaredProceduralTime,
};
use infrastructure::{PostgresProceduralResourceStore, RingSha256Hasher};
use std::sync::Arc;

pub fn store(db: &Fixture) -> Arc<PostgresProceduralResourceStore> {
    Arc::new(
        PostgresProceduralResourceStore::open(
            &db.runtime_url,
            Arc::new(RingSha256Hasher),
            Arc::new(FixedClock(db.at)),
        )
        .unwrap(),
    )
}
pub fn service(db: &Fixture, actor: UserId, role: Role) -> ProceduralResourceService {
    service_with_format(db, actor, role, Arc::new(FormatCheck(None)))
}
pub fn service_with_format(
    db: &Fixture,
    actor: UserId,
    role: Role,
    format: Arc<dyn application::documents::DocumentFormatBatchValidator>,
) -> ProceduralResourceService {
    ProceduralResourceService::new(
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
        processor(),
        format,
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}
pub fn text(value: &str) -> FactText {
    FactText::new(value).unwrap()
}
pub fn label(value: &str) -> FactLabel {
    FactLabel::new(value).unwrap()
}
pub fn registration(db: &Fixture) -> ResourceCommand {
    let fact = facts::persist(
        &facts::service(db, db.owner, Role::Owner),
        db.case,
        facts::record(),
    );
    let record = super::case_stage_database_support::upload(db, db.case, "resolution.pdf");
    ResourceCommand {
        operation_id: ResourceOperationId::new(),
        resource_id: ResourceId::new(),
        change: ResourceChange::Register {
            values: ResourceValues::new(ResourceValuesInput {
                kind: ResourceKind::Appeal,
                mode: FactDeclaration::Known(ResourceMode::Written),
                title: label("Declared appeal"),
                resolution: facts::resolution_ref(&fact),
                resolution_evidence: FactEvidence::new(
                    domain::crypto::DocumentVersionRef {
                        id: record.id,
                        version: record.version,
                    },
                    record.digest,
                    label("Page one"),
                ),
                resolution_reference: FactDeclaration::Unknown(text("Not stated")),
                issuing_authority: FactDeclaration::Unknown(text("Not stated")),
                receiving_authority: None,
                resolution_at: DeclaredProceduralTime::unknown(),
                notification_at: None,
                challenged_part: text("Declared challenged part"),
                grounds: text("Declared grounds"),
                appellants: vec![ResourceAppellant::new(
                    label("Declared appellant"),
                    FactDeclaration::Unknown(text("Role not stated")),
                    None,
                )],
            })
            .unwrap(),
        },
    }
}
pub fn persist(
    service: &ProceduralResourceService,
    case: CaseId,
    command: ResourceCommand,
) -> ResourceDetail {
    let draft = service.prepare("session", case, command.clone()).unwrap();
    service
        .submit("session", case, command, draft.submission_digest)
        .unwrap()
}
pub fn act(base: &ResourceDetail) -> ResourceCommand {
    let source = &base.sources.supports[0];
    ResourceCommand {
        operation_id: ResourceOperationId::new(),
        resource_id: base.id,
        change: ResourceChange::RecordAct {
            expected_revision: base.revision,
            act_id: ResourceActId::new(),
            values: ResourceActValues::new(ResourceActValuesInput {
                kind: ResourceActKind::Interposition,
                mode: FactDeclaration::Known(ResourceMode::Written),
                occurred_at: DeclaredProceduralTime::unknown(),
                authority: FactDeclaration::Unknown(text("Not stated")),
                statement: text("Declared filing"),
                evidence: vec![FactEvidence::new(
                    source.reference,
                    source.digest,
                    label("Page two"),
                )],
            })
            .unwrap(),
        },
    }
}
pub fn snapshot(db: &mut Fixture) -> serde_json::Value {
    db.admin.query_one("SELECT jsonb_build_object('roots',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM case_procedural_resources r),'revisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY resource_id,revision) FROM case_procedural_resource_revisions r),'acts',(SELECT jsonb_agg(to_jsonb(a) ORDER BY id) FROM case_procedural_resource_acts a),'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a WHERE action<>'procedural_resource.prepare'))", &[]).unwrap().get(0)
}
