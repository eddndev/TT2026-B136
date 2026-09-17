mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod procedural_fact_backend_support;

use application::procedural_facts::*;
use domain::identity::Role;
use infrastructure::{PostgresCaseRepository, RingSha256Hasher};
use procedural_fact_backend_support::*;
use std::sync::Arc;
use uuid::Uuid;

fn rejects(db: &Fixture) {
    assert!(PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher)).is_err());
}

#[test]
fn startup_rejects_fact_roots_without_their_first_revision() {
    let Some(mut db) = Fixture::new() else { return };
    db.admin
        .batch_execute("ALTER TABLE case_procedural_facts DISABLE TRIGGER ALL")
        .unwrap();
    db.admin
        .execute(
            "INSERT INTO case_procedural_facts(family,id,case_id) VALUES('resolution',$1,$2)",
            &[&Uuid::nil(), &db.case.as_uuid()],
        )
        .unwrap();
    db.admin
        .batch_execute("ALTER TABLE case_procedural_facts ENABLE TRIGGER ALL")
        .unwrap();
    rejects(&db);
}

#[test]
fn startup_checks_exact_baseline_for_both_families_with_the_same_nil_uuid() {
    for family in ["resolution", "notification"] {
        let Some(mut db) = Fixture::new() else { return };
        let svc = service(&db, db.owner, Role::Owner);
        let resolution = persist(
            &svc,
            db.case,
            ProceduralFactCommand::Resolution(ResolutionCommand::new(
                FactOperationId::new(),
                ResolutionId::from_uuid(Uuid::nil()),
                FactChange::record(values("Nil resolution")),
            )),
        );
        let parent = resolution_ref(&resolution);
        persist(
            &svc,
            db.case,
            ProceduralFactCommand::Notification(
                NotificationCommand::new(
                    FactOperationId::new(),
                    NotificationId::from_uuid(Uuid::nil()),
                    parent.id,
                    FactChange::record(notification_values(parent, "Nil notification")),
                )
                .unwrap(),
            ),
        );
        db.store();
        db.admin.batch_execute(
            "ALTER TABLE case_procedural_fact_revisions DISABLE TRIGGER procedural_fact_immutable"
        ).unwrap();
        db.admin.execute(
            "UPDATE case_procedural_fact_revisions SET recorded_administration_title='Substituted baseline'
             WHERE family=$1 AND id=$2", &[&family, &Uuid::nil()],
        ).unwrap();
        db.admin.batch_execute(
            "ALTER TABLE case_procedural_fact_revisions ENABLE TRIGGER procedural_fact_immutable"
        ).unwrap();
        rejects(&db);
    }
}

#[test]
fn startup_rejects_gaps_in_each_fact_family_history() {
    for notification in [false, true] {
        let Some(mut db) = Fixture::new() else { return };
        let svc = service(&db, db.owner, Role::Owner);
        let parent = persist(&svc, db.case, record());
        let first = if notification {
            persist(&svc, db.case, notify(resolution_ref(&parent)))
        } else {
            parent
        };
        let second = persist(&svc, db.case, correct(&first));
        persist(&svc, db.case, withdraw(&second));
        let (family, id) = match first.snapshot.target() {
            FactTarget::Resolution(id) => ("resolution", id.as_uuid()),
            FactTarget::Notification { id, .. } => ("notification", id.as_uuid()),
        };
        db.admin
            .batch_execute("ALTER TABLE case_procedural_fact_revisions DISABLE TRIGGER ALL")
            .unwrap();
        db.admin.execute(
            "DELETE FROM case_procedural_fact_revisions WHERE family=$1 AND id=$2 AND revision=2",
            &[&family, &id],
        ).unwrap();
        db.admin
            .batch_execute("ALTER TABLE case_procedural_fact_revisions ENABLE TRIGGER ALL")
            .unwrap();
        rejects(&db);
    }
}

#[test]
fn startup_validates_fact_sources_beyond_the_first_inventory_page() {
    let Some(mut db) = Fixture::new() else { return };
    let svc = service(&db, db.owner, Role::Owner);
    for number in 1..=65_u128 {
        persist(
            &svc,
            db.case,
            ProceduralFactCommand::Resolution(ResolutionCommand::new(
                FactOperationId::new(),
                ResolutionId::from_uuid(Uuid::from_u128(number)),
                FactChange::record(values("Paginated inventory")),
            )),
        );
    }
    db.store();
    db.admin
        .batch_execute(
            "ALTER TABLE case_procedural_fact_revisions DISABLE TRIGGER procedural_fact_immutable",
        )
        .unwrap();
    db.admin.execute(
        "UPDATE case_procedural_fact_revisions SET recorded_administration_reference='Substituted'
         WHERE family='resolution' AND id=$1", &[&Uuid::from_u128(65)],
    ).unwrap();
    db.admin
        .batch_execute(
            "ALTER TABLE case_procedural_fact_revisions ENABLE TRIGGER procedural_fact_immutable",
        )
        .unwrap();
    rejects(&db);
}
