use super::*;

#[test]
fn authorized_roles_revalidate_activity_and_membership_before_even_empty_results() {
    let Some(mut db) = Fixture::new() else { return };
    let captures = Captures::new(&mut db);
    let adapter = store(&db);
    let member = db.user("litigator", true);
    let assistant = db.user("paralegal", true);
    let client = db.user("client", true);
    let outsider = db.user("litigator", false);
    for actor in [db.owner, member, assistant] {
        assert!(adapter
            .list_for_target(
                actor,
                db.case,
                target(&captures),
                query(10, None, None),
                db.at
            )
            .unwrap()
            .associations
            .is_empty());
    }
    let before = business_and_audit(&mut db);
    for case in [db.case, CaseId::new()] {
        assert!(matches!(
            adapter.list_for_target(
                client,
                case,
                target(&captures),
                query(10, None, None),
                db.at
            ),
            Err(ApplicationError::PermissionDenied)
        ));
        assert!(matches!(
            adapter.list_for_target(
                outsider,
                case,
                target(&captures),
                query(10, None, None),
                db.at
            ),
            Err(ApplicationError::CaseNotFound)
        ));
    }
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE user_id=$1",
            &[&assistant.as_uuid()],
        )
        .unwrap();
    assert!(matches!(
        adapter.list_for_target(
            assistant,
            db.case,
            target(&captures),
            query(10, None, None),
            db.at
        ),
        Err(ApplicationError::CaseNotFound)
    ));
    db.admin
        .execute(
            "UPDATE users SET active=false,revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1",
            &[&member.as_uuid()],
        )
        .unwrap();
    assert!(adapter
        .list_for_target(
            member,
            db.case,
            target(&captures),
            query(10, None, None),
            db.at
        )
        .is_err());
    assert_eq!(business_and_audit(&mut db), before);
}

#[test]
fn rejected_inverse_read_audit_returns_no_page_and_rolls_back_its_event() {
    let Some(mut db) = Fixture::new() else { return };
    let captures = Captures::new(&mut db);
    let workflow = service(&db, db.owner, Role::Owner);
    let linked = persist(&workflow, db.case, captures.resource.id, captures.link());
    let prior = workflow
        .list_for_target("session", db.case, target(&captures), query(10, None, None))
        .unwrap();
    assert_eq!(prior.associations[0].association, linked);
    db.admin.batch_execute("CREATE FUNCTION reject_target_read_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected inverse read audit failure'; END; $$; CREATE TRIGGER reject_target_read_audit BEFORE INSERT ON audit_events FOR EACH ROW WHEN (NEW.action='resource_activity.target_list') EXECUTE FUNCTION reject_target_read_audit()").unwrap();
    let before = business_and_audit(&mut db);
    let independent = independent_rows(&mut db);
    assert!(matches!(
        workflow.list_for_target("session", db.case, target(&captures), query(10, None, None)),
        Err(ApplicationError::Port(_))
    ));
    assert_eq!(business_and_audit(&mut db), before);
    assert_eq!(independent_rows(&mut db), independent);
    db.admin.batch_execute("DROP TRIGGER reject_target_read_audit ON audit_events; DROP FUNCTION reject_target_read_audit()").unwrap();
    let recovered = workflow
        .list_for_target("session", db.case, target(&captures), query(10, None, None))
        .unwrap();
    assert_eq!(recovered.associations[0].association, linked);
}
