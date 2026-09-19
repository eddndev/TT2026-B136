mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_profile_database_support;
mod deadline_restore_support;
mod procedural_fact_backend_support;
use application::{deadline_profiles::DeadlineProfileCollection, deadlines::*};
use deadline_backend_support::*;
use deadline_profile_database_support as profiles;
use deadline_restore_support::{dump_restore, expressions, inventory_snapshot};
use domain::identity::Role;
use infrastructure::RingSha256Hasher;
use procedural_fact_backend_support as facts;

#[test]
fn dump_restore_preserves_deadline_captures_after_revocation_and_new_source_heads() {
    let Some(mut db) = Fixture::new() else { return };
    let reader = db.user("owner", false);
    let responsible = db.user("paralegal", true);
    let responsible_email: String = db
        .admin
        .query_one(
            "SELECT email FROM users WHERE id=$1",
            &[&responsible.as_uuid()],
        )
        .unwrap()
        .get(0);
    let workflow = service(&db, db.owner, Role::Owner);
    let profile = profile(&db);
    let source = source(&db);
    let mut initial = command(&db, &profile, &source);
    definition_mut(&mut initial).responsible = responsible;
    let first = persist_legacy(&db, db.owner, initial);
    let second = persist_legacy(&db, db.owner, correct(&first));
    let third = persist_legacy(&db, db.owner, attention(&second));
    let fourth = persist_legacy(&db, db.owner, retire(&third));
    let mut next = command(&db, &profile, &source);
    definition_mut(&mut next).responsible = responsible;
    let active = persist_legacy(&db, db.owner, next);
    let history = vec![first, second, third, fourth];
    facts::persist(
        &facts::service(&db, db.owner, Role::Owner),
        db.case,
        facts::withdraw(&source),
    );
    profiles::persist(
        &profiles::service(&db, db.owner, Role::Owner),
        DeadlineProfileCollection::ForCase(db.case),
        profiles::retire(&profile),
    );
    db.admin
        .execute(
            "UPDATE users SET active=FALSE,email='former-author@example.test' WHERE id=$1",
            &[&db.owner.as_uuid()],
        )
        .unwrap();
    db.admin.execute("UPDATE users SET active=FALSE,email='former-responsible@example.test',role='client' WHERE id=$1",
        &[&responsible.as_uuid()]).unwrap();
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE user_id=$1",
            &[&responsible.as_uuid()],
        )
        .unwrap();
    drop(workflow);
    let before = inventory_snapshot(&mut db);
    db.migrate();
    assert_eq!(inventory_snapshot(&mut db), before);
    let expressions_before = expressions(&mut db);
    dump_restore(&mut db);
    assert_eq!(inventory_snapshot(&mut db), before);
    let expressions_after = expressions(&mut db);
    let changed: Vec<_> = expressions_before
        .iter()
        .filter(|(name, value)| expressions_after.get(*name) != Some(*value))
        .map(|(name, value)| (name, value, expressions_after.get(name)))
        .collect();
    assert!(
        changed.is_empty(),
        "restoration changed catalog expressions: {changed:?}"
    );
    let restored = store(&db);
    // Opening validates inventory without appending read audit or recalculating history.
    assert_eq!(inventory_snapshot(&mut db), before);
    for exact in &history {
        assert_eq!(
            restored
                .get(reader, db.case, exact.id, Some(exact.revision), db.at)
                .unwrap(),
            *exact
        );
        assert_eq!(exact.recorded_by.user_id(), Some(db.owner));
        assert_eq!(exact.recorded_by.email(), Some("owner@example.test"));
        assert_eq!(exact.responsible.id, responsible);
        assert_eq!(exact.responsible.role, Role::Paralegal);
        assert_eq!(exact.responsible.email, responsible_email);
        deadline_receipt_matches(&RingSha256Hasher, exact).unwrap();
    }
    let latest = history.last().unwrap();
    assert_eq!(
        restored
            .get(reader, db.case, latest.id, None, db.at)
            .unwrap(),
        *latest
    );
    assert_eq!(
        restored
            .history(reader, db.case, latest.id, history_query(20, None), db.at)
            .unwrap()
            .revisions,
        history
            .iter()
            .rev()
            .map(|value| DeadlineHistoryEntry::from_detail(&RingSha256Hasher, value).unwrap())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        restored
            .get(reader, db.case, active.id, None, db.at)
            .unwrap(),
        active
    );
    let successor = persist(
        &service(&db, reader, Role::Owner),
        db.case,
        human(attention(&active), None),
    );
    assert_eq!(successor.revision.get(), 2);
    assert_eq!(successor.recorded_by.user_id(), Some(reader));
    assert_eq!(successor.definition, active.definition);
    assert_eq!(successor.calculation, active.calculation);
    assert_eq!(successor.responsible, active.responsible);
    assert_eq!(
        successor.calculation.result.due_at(),
        active.calculation.result.due_at()
    );
    let audit: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM audit_events WHERE action='deadline.attention_recorded'",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(audit, 2);
}
