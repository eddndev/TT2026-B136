use crate::case_administration_support::Fixture;
use crate::case_stage_database_support as stages;
use crate::hearing_fixture::{creation, principal, FixedClock, TestIdentity};
use application::case_stages::{CaseStageEntry, CaseStageService, CaseStageWorkflow};
use application::cases::{CaseAdministrativeStatus, CaseRepository, CaseRevisionExpectation};
use application::identity::Principal;
use application::precautionary_hearings::{
    PrecautionaryContext, PrecautionaryContextMaterial, PrecautionaryContextReadService,
    PrecautionaryContextReadStore, PrecautionaryContextReadWorkflow,
};
use domain::case_stages::{CaseStageRevision, DeclaredStageTime, StageTransition};
use domain::cases::CaseId;
use domain::identity::Role;
use infrastructure::{PostgresPrecautionaryHearingStore, RingSha256Hasher};
use std::sync::Arc;
use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};

#[test]
fn context_reads_return_current_revisions_and_exact_active_stage_origin_after_closure() {
    let Some(mut db) = Fixture::new() else { return };
    let (actor, first) = setup(&mut db);
    let read_at = db.at + Duration::seconds(10);
    let storage = context_store(&db, read_at);
    let reader = context_reader(storage, actor.clone(), read_at);
    assert_eq!(reader.get("session", db.case).unwrap(), first);

    db.at += Duration::seconds(1);
    let detail = db
        .store()
        .replace_administration(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(1),
            creation("Updated context identifiers")
                .into_values()
                .editable()
                .clone(),
            db.at,
        )
        .unwrap();
    let stage_administration = detail.administration.snapshot().unwrap().clone();
    let support = stages::upload(&db, db.case, "context-accusation.pdf");
    let workflow = CaseStageService::new(
        stages::store(&db),
        Arc::new(TestIdentity(actor.clone())),
        stages::processor(),
        Arc::new(stages::FormatCheck(None)),
        Arc::new(FixedClock(db.at)),
    );
    let stage = workflow
        .transition(
            "session",
            db.case,
            CaseStageRevision::FIRST,
            StageTransition::to_intermediate(
                DeclaredStageTime::instant(db.at).unwrap(),
                stages::reference(&support),
                None,
            ),
        )
        .unwrap()
        .current
        .entry()
        .unwrap()
        .clone();
    let active = PrecautionaryContext::new(
        &RingSha256Hasher,
        PrecautionaryContextMaterial {
            case_id: db.case,
            administration: stage_administration.clone(),
            stage: stage.clone(),
            stage_administration: stage_administration.clone(),
        },
    )
    .unwrap();
    assert_eq!(reader.get("session", db.case).unwrap(), active);

    db.at += Duration::seconds(1);
    let closed = db
        .store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::Revision(stage_administration.revision),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    let expected = PrecautionaryContext::new(
        &RingSha256Hasher,
        PrecautionaryContextMaterial {
            case_id: db.case,
            administration: closed.administration.snapshot().unwrap().clone(),
            stage,
            stage_administration,
        },
    )
    .unwrap();
    let observed = reader.get("session", db.case).unwrap();
    assert_eq!(observed, expected);
    assert_eq!(observed.material().administration.revision.get(), 3);
    assert_eq!(observed.material().stage.stage_revision().get(), 2);
    assert_eq!(observed.material().stage_administration.revision.get(), 2);
    assert_eq!(
        observed.material().administration.values.status(),
        CaseAdministrativeStatus::Closed
    );
    assert_eq!(
        observed.material().stage_administration.values.status(),
        CaseAdministrativeStatus::Active
    );
    assert_eq!(
        observed.digest(&RingSha256Hasher),
        expected.digest(&RingSha256Hasher)
    );
    let events = db.admin.query(
        "SELECT actor,resource,timestamp FROM audit_events WHERE action='precautionary_context.read' ORDER BY sequence",
        &[],
    ).unwrap();
    assert_eq!(events.len(), 3);
    for (event, context) in events.iter().zip([first, active, expected]) {
        assert_eq!(event.get::<_, String>(0), actor.email);
        assert_eq!(event.get::<_, String>(1), marker(&context));
        let timestamp: String = event.get(2);
        assert_eq!(
            OffsetDateTime::parse(&timestamp, &Rfc3339).unwrap(),
            read_at
        );
    }
}

#[test]
fn context_reads_revalidate_full_staff_identity_and_current_case_membership() {
    let Some(mut db) = Fixture::new() else { return };
    let (_, expected) = setup(&mut db);
    let storage = context_store(&db, db.at);
    for role in ["litigator", "paralegal"] {
        let user = db.user(role, true);
        let actor = principal(&mut db, user);
        let reader = context_reader(storage.clone(), actor.clone(), db.at);
        assert_eq!(reader.get("session", db.case).unwrap(), expected);
        let stale = Principal {
            email: "old-context-reader@example.test".into(),
            ..actor.clone()
        };
        let stale_role = Principal {
            role: Role::Owner,
            ..actor.clone()
        };
        let before = snapshot(&mut db);
        for invalid in [&stale, &stale_role] {
            assert!(
                PrecautionaryContextReadStore::get(storage.as_ref(), invalid, db.case).is_err()
            );
        }
        assert!(reader.get("session", CaseId::new()).is_err());
        assert_eq!(snapshot(&mut db), before);
        db.admin
            .execute(
                "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
                &[&db.case.as_uuid(), &user.as_uuid()],
            )
            .unwrap();
        let before = snapshot(&mut db);
        assert!(reader.get("session", db.case).is_err());
        assert_eq!(snapshot(&mut db), before);
    }
    let client = db.user("client", true);
    let client = principal(&mut db, client);
    let before = snapshot(&mut db);
    assert!(PrecautionaryContextReadStore::get(storage.as_ref(), &client, db.case).is_err());
    assert!(context_reader(storage, client, db.at)
        .get("session", db.case)
        .is_err());
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn context_read_audit_failure_and_corrupt_source_disclose_nothing_and_write_nothing() {
    let Some(mut db) = Fixture::new() else { return };
    let (actor, expected) = setup(&mut db);
    let reader = context_reader(context_store(&db, db.at), actor, db.at);
    assert_eq!(reader.get("session", db.case).unwrap(), expected);
    db.admin.batch_execute("CREATE FUNCTION reject_context_read() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN IF NEW.action='precautionary_context.read' THEN RAISE EXCEPTION 'injected context audit failure';
        END IF; RETURN NEW; END; $$;
        CREATE TRIGGER reject_context_read BEFORE INSERT ON audit_events
        FOR EACH ROW EXECUTE FUNCTION reject_context_read()").unwrap();
    let before = snapshot(&mut db);
    assert!(reader.get("session", db.case).is_err());
    assert_eq!(snapshot(&mut db), before);
    db.admin
        .batch_execute(
            "DROP TRIGGER reject_context_read ON audit_events;
        DROP FUNCTION reject_context_read()",
        )
        .unwrap();
    assert_eq!(reader.get("session", db.case).unwrap(), expected);

    let mut tx = db.admin.transaction().unwrap();
    tx.batch_execute("ALTER TABLE case_administration_revisions DISABLE TRIGGER ALL")
        .unwrap();
    tx.execute(
        "UPDATE case_administration_revisions SET changed_at='2025-01-01T01:00:00.123456789+01:00' WHERE case_id=$1",
        &[&db.case.as_uuid()],
    ).unwrap();
    tx.batch_execute("ALTER TABLE case_administration_revisions ENABLE TRIGGER ALL")
        .unwrap();
    tx.commit().unwrap();
    let before = snapshot(&mut db);
    assert!(reader.get("session", db.case).is_err());
    assert_eq!(snapshot(&mut db), before);
}

fn setup(db: &mut Fixture) -> (Principal, PrecautionaryContext) {
    db.case = CaseId::new();
    let detail = db
        .store()
        .register_penal(db.owner, db.case, creation(&db.case.to_string()), db.at)
        .unwrap();
    let administration = detail.administration.snapshot().unwrap().clone();
    let context = PrecautionaryContext::new(
        &RingSha256Hasher,
        PrecautionaryContextMaterial {
            case_id: db.case,
            administration: administration.clone(),
            stage: CaseStageEntry::Initial(detail.initial_stage.unwrap()),
            stage_administration: administration,
        },
    )
    .unwrap();
    let owner = db.owner;
    (principal(db, owner), context)
}

fn context_store(db: &Fixture, at: OffsetDateTime) -> Arc<PostgresPrecautionaryHearingStore> {
    Arc::new(
        PostgresPrecautionaryHearingStore::open(
            &db.runtime_url,
            Arc::new(RingSha256Hasher),
            Arc::new(FixedClock(at)),
        )
        .unwrap(),
    )
}

fn context_reader(
    storage: Arc<PostgresPrecautionaryHearingStore>,
    actor: Principal,
    at: OffsetDateTime,
) -> PrecautionaryContextReadService {
    PrecautionaryContextReadService::new(
        storage,
        Arc::new(TestIdentity(actor)),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(at)),
    )
}

fn marker(context: &PrecautionaryContext) -> String {
    let material = context.material();
    format!(
        "case:{}:administration:{}:stage:{}:context:{}",
        material.case_id,
        material.administration.revision.get(),
        material.stage.stage_revision().get(),
        context.digest(&RingSha256Hasher).to_hex()
    )
}

fn snapshot(db: &mut Fixture) -> serde_json::Value {
    serde_json::json!({"administration": db.snapshot(), "stage": stages::snapshot(db)})
}
