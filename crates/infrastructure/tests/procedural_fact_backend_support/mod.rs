#![allow(dead_code)]

pub use super::case_administration_support::Fixture;
use super::case_stage_database_support::{processor, FixedClock, FormatCheck, TestIdentity};
use application::{identity::Principal, procedural_facts::*};
use domain::{
    cases::CaseId,
    clock::Clock,
    identity::{Role, UserId},
    procedural_time::DeclaredProceduralTime,
};
use infrastructure::{procedural_fact_postgres::PostgresProceduralFactStore, RingSha256Hasher};
use std::sync::Arc;

pub fn store(db: &Fixture) -> Arc<PostgresProceduralFactStore> {
    clocked_store(db, Arc::new(FixedClock(db.at)))
}
pub fn clocked_store(
    db: &Fixture,
    clock: Arc<dyn Clock + Send + Sync>,
) -> Arc<PostgresProceduralFactStore> {
    Arc::new(
        PostgresProceduralFactStore::open(&db.runtime_url, Arc::new(RingSha256Hasher), clock)
            .unwrap(),
    )
}
pub fn service(db: &Fixture, actor: UserId, role: Role) -> ProceduralFactService {
    service_with_store(db, store(db), actor, role)
}
pub fn service_with_store(
    db: &Fixture,
    store: Arc<dyn ProceduralFactStore>,
    actor: UserId,
    role: Role,
) -> ProceduralFactService {
    ProceduralFactService::new(
        store,
        Arc::new(TestIdentity(Principal {
            id: actor,
            email: "session@example.test".into(),
            role,
        })),
        processor(),
        Arc::new(FormatCheck(None)),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
}
pub fn text(value: &str) -> FactText {
    FactText::new(value).unwrap()
}
pub fn values(summary: &str) -> ResolutionValues {
    ResolutionValues::new(ResolutionValuesInput {
        class: FactDeclaration::Known(ResolutionClass::Order),
        subtype: None,
        issuer: FactDeclaration::Unknown(text("Issuer not stated")),
        issued_at: DeclaredProceduralTime::unknown(),
        summary: text(summary),
        provenance: FactProvenance::OperatorNote {
            note: text("Operator declaration"),
        },
    })
}
pub fn notification_values(parent: FactResolutionRef, summary: &str) -> NotificationValues {
    NotificationValues::new(NotificationValuesInput {
        resolution: parent,
        character: FactDeclaration::Known(NotificationCharacter::Personal),
        medium: FactDeclaration::Known(NotificationMedium::InPerson),
        context: FactDeclaration::Known(NotificationContext::OutsideHearing),
        outcome: FactDeclaration::Known(NotificationOutcome::Attempted),
        subtype: None,
        practiced_at: DeclaredProceduralTime::unknown(),
        received_at: None,
        stated_effect: None,
        intended_recipient: FactDeclaration::Unknown(text("Recipient not stated")),
        actual_receiver: FactDeclaration::Unknown(text("Receiver not stated")),
        representation: FactRepresentation::NotRecorded(text("Representation not stated")),
        summary: text(summary),
        provenance: FactProvenance::OperatorNote {
            note: text("Operator declaration"),
        },
    })
    .unwrap()
}
pub fn record() -> ProceduralFactCommand {
    ProceduralFactCommand::Resolution(ResolutionCommand::new(
        FactOperationId::new(),
        ResolutionId::new(),
        FactChange::record(values("Declared resolution")),
    ))
}
pub fn resolution_ref(detail: &FactDetail) -> FactResolutionRef {
    let FactTarget::Resolution(id) = detail.snapshot.target() else {
        panic!("resolution expected")
    };
    FactResolutionRef {
        id,
        revision: detail.snapshot.metadata().revision,
    }
}
pub fn notify(reference: FactResolutionRef) -> ProceduralFactCommand {
    ProceduralFactCommand::Notification(
        NotificationCommand::new(
            FactOperationId::new(),
            NotificationId::new(),
            reference.id,
            FactChange::record(notification_values(reference, "Declared notification")),
        )
        .unwrap(),
    )
}
pub fn correct(detail: &FactDetail) -> ProceduralFactCommand {
    let revision = detail.snapshot.metadata().revision;
    match &detail.snapshot {
        ProceduralFactSnapshot::Resolution(snapshot) => {
            let current = &snapshot.values;
            let replacement = ResolutionValues::new(ResolutionValuesInput {
                class: current.class().clone(),
                subtype: current.subtype().cloned(),
                issuer: current.issuer().clone(),
                issued_at: current.issued_at(),
                summary: text("Corrected resolution"),
                provenance: current.provenance().clone(),
            });
            ProceduralFactCommand::Resolution(ResolutionCommand::new(
                FactOperationId::new(),
                snapshot.root.id(),
                FactChange::correct(revision, replacement, text("Correct transcription")),
            ))
        }
        ProceduralFactSnapshot::Notification(snapshot) => ProceduralFactCommand::Notification(
            NotificationCommand::new(
                FactOperationId::new(),
                snapshot.root.id(),
                snapshot.root.resolution_id(),
                FactChange::correct(
                    revision,
                    notification_values(snapshot.values.resolution(), "Corrected notification"),
                    text("Correct transcription"),
                ),
            )
            .unwrap(),
        ),
    }
}
pub fn withdraw(detail: &FactDetail) -> ProceduralFactCommand {
    let revision = detail.snapshot.metadata().revision;
    match detail.snapshot.target() {
        FactTarget::Resolution(id) => ProceduralFactCommand::Resolution(ResolutionCommand::new(
            FactOperationId::new(),
            id,
            FactChange::withdraw(revision, text("Withdraw declaration")),
        )),
        FactTarget::Notification { id, resolution_id } => ProceduralFactCommand::Notification(
            NotificationCommand::new(
                FactOperationId::new(),
                id,
                resolution_id,
                FactChange::withdraw(revision, text("Withdraw declaration")),
            )
            .unwrap(),
        ),
    }
}
pub fn persist(
    service: &ProceduralFactService,
    case: CaseId,
    command: ProceduralFactCommand,
) -> FactDetail {
    let prepared = service.prepare("session", case, command.clone()).unwrap();
    service
        .submit("session", case, command, prepared.submission_digest)
        .unwrap()
}
pub fn counts(db: &mut Fixture) -> (i64, i64, i64) {
    let row = db.admin.query_one("SELECT (SELECT count(*) FROM case_procedural_facts),(SELECT count(*) FROM case_procedural_fact_revisions),(SELECT count(*) FROM audit_events)", &[]).unwrap();
    (row.get(0), row.get(1), row.get(2))
}
pub fn snapshot(db: &mut Fixture) -> serde_json::Value {
    db.admin.query_one("SELECT jsonb_build_object('roots',(SELECT jsonb_agg(to_jsonb(f) ORDER BY family,id) FROM case_procedural_facts f),'revisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY family,id,revision) FROM case_procedural_fact_revisions r),'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))", &[]).unwrap().get(0)
}
