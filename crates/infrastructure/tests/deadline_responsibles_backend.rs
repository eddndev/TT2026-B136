mod case_administration_support;
mod deadline_responsibles_support;
use application::{cases::*, deadlines::*, ApplicationError};
use deadline_responsibles_support::*;
use domain::{
    cases::CaseId,
    identity::{Role, UserId},
};
use uuid::Uuid;

fn query(limit: u32, after: Option<UserId>) -> DeadlineResponsibleQuery {
    DeadlineResponsibleQuery::new(limit, after).unwrap()
}

#[test]
fn responsible_candidates_are_active_case_staff_and_global_owners_without_duplicates_or_secrets() {
    let Some(mut db) = Fixture::new() else { return };
    let owner2 = db.user("owner", false);
    let owner3 = db.user("owner", true);
    let litigator = db.user("litigator", true);
    let paralegal = db.user("paralegal", true);
    let excluded = [
        db.user("client", true),
        db.user("litigator", false),
        db.user("paralegal", false),
        db.user("owner", false),
        db.user("paralegal", true),
    ];
    for id in &excluded[3..] {
        db.admin
            .execute(
                "UPDATE users SET active=false WHERE id=$1",
                &[&id.as_uuid()],
            )
            .unwrap();
    }
    let mut expected = vec![db.owner, owner2, owner3, litigator, paralegal];
    expected.sort_by_key(|id| id.as_uuid());
    let adapter = store(&db);
    for actor in [db.owner, litigator, paralegal] {
        let page = adapter
            .responsibles(actor, db.case, query(100, None), db.at)
            .unwrap();
        assert_eq!(page.case_id, db.case);
        assert_eq!(
            page.responsibles
                .iter()
                .map(|row| row.id)
                .collect::<Vec<_>>(),
            expected
        );
        assert!(!page.has_more);
        assert_eq!(page.next_after_id, None);
        for row in page.responsibles {
            assert!(!row.email.trim().is_empty());
            assert!(matches!(
                row.role,
                Role::Owner | Role::Litigator | Role::Paralegal
            ));
        }
    }
    assert_eq!(
        db.admin
            .query_one(
                "SELECT count(*) FROM audit_events WHERE action='deadline.responsibles_read'",
                &[]
            )
            .unwrap()
            .get::<_, i64>(0),
        3
    );
    let other_case = CaseId::new();
    db.admin.execute("INSERT INTO cases(id,title,reference,created_by,required_initial_revision) VALUES($1,'Other case','OTHER',$2,NULL)", &[&other_case.as_uuid(), &db.owner.as_uuid()]).unwrap();
    let other = adapter
        .responsibles(db.owner, other_case, query(100, None), db.at)
        .unwrap();
    assert_eq!(other.responsibles.len(), 3);
    assert!(other.responsibles.iter().all(|row| row.role == Role::Owner));
    let before = snapshot(&mut db);
    assert!(matches!(
        adapter.responsibles(litigator, other_case, query(100, None), db.at),
        Err(ApplicationError::CaseNotFound)
    ));
    for actor in [excluded[0], excluded[1], excluded[3], UserId::new()] {
        let result = adapter.responsibles(actor, db.case, query(100, None), db.at);
        if actor == excluded[0] {
            assert!(matches!(result, Err(ApplicationError::PermissionDenied)));
        } else if actor == excluded[1] {
            assert!(matches!(result, Err(ApplicationError::CaseNotFound)));
        } else {
            assert!(matches!(result, Err(ApplicationError::InvalidSession)));
        }
    }
    assert!(matches!(
        adapter.responsibles(db.owner, CaseId::new(), query(100, None), db.at),
        Err(ApplicationError::CaseNotFound)
    ));
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn responsible_keyset_pages_are_exclusive_and_do_not_require_an_existing_cursor() {
    let Some(mut db) = Fixture::new() else { return };
    let mut expected = vec![db.owner];
    for _ in 0..4 {
        expected.push(db.user("paralegal", true));
    }
    expected.sort_by_key(|id| id.as_uuid());
    let adapter = store(&db);
    let mut cursor = Some(UserId::from_uuid(Uuid::nil()));
    let mut seen = Vec::new();
    loop {
        let page = adapter
            .responsibles(db.owner, db.case, query(2, cursor), db.at)
            .unwrap();
        seen.extend(page.responsibles.iter().map(|row| row.id));
        if !page.has_more {
            assert_eq!(page.next_after_id, None);
            break;
        }
        assert_eq!(page.responsibles.len(), 2);
        assert_eq!(
            page.next_after_id,
            page.responsibles.last().map(|row| row.id)
        );
        cursor = page.next_after_id;
    }
    assert_eq!(seen, expected);
    let absent = UserId::from_uuid(Uuid::from_u128(expected[1].as_uuid().as_u128() + 1));
    let page = adapter
        .responsibles(db.owner, db.case, query(100, Some(absent)), db.at)
        .unwrap();
    assert_eq!(
        page.responsibles
            .iter()
            .map(|row| row.id)
            .collect::<Vec<_>>(),
        expected
            .into_iter()
            .filter(|id| id.as_uuid() > absent.as_uuid())
            .collect::<Vec<_>>()
    );
    let page = adapter
        .responsibles(
            db.owner,
            db.case,
            query(100, Some(UserId::from_uuid(Uuid::max()))),
            db.at,
        )
        .unwrap();
    assert!(page.responsibles.is_empty());
    assert!(!page.has_more);
}

#[test]
fn responsible_reads_survive_case_closure_and_recheck_membership_and_candidate_revocation() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&db);
    let paralegal = db.user("paralegal", true);
    let candidate = db.user("litigator", true);
    let adapter = store(&db);
    assert!(adapter
        .responsibles(paralegal, db.case, query(100, None), db.at)
        .unwrap()
        .responsibles
        .iter()
        .any(|row| row.id == candidate));
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(1),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    assert!(adapter
        .responsibles(paralegal, db.case, query(100, None), db.at)
        .is_ok());
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
            &[&db.case.as_uuid(), &candidate.as_uuid()],
        )
        .unwrap();
    assert!(!adapter
        .responsibles(paralegal, db.case, query(100, None), db.at)
        .unwrap()
        .responsibles
        .iter()
        .any(|row| row.id == candidate));
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
            &[&db.case.as_uuid(), &paralegal.as_uuid()],
        )
        .unwrap();
    let before = snapshot(&mut db);
    assert!(matches!(
        adapter.responsibles(paralegal, db.case, query(100, None), db.at),
        Err(ApplicationError::CaseNotFound)
    ));
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn responsible_failed_audit_prevents_return_and_preserves_database() {
    let Some(mut db) = Fixture::new() else { return };
    db.admin.batch_execute("CREATE FUNCTION reject_responsible_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected audit failure'; END; $$; CREATE TRIGGER reject_responsible_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION reject_responsible_audit()").unwrap();
    let before = snapshot(&mut db);
    assert!(store(&db)
        .responsibles(db.owner, db.case, query(100, None), db.at)
        .is_err());
    assert_eq!(snapshot(&mut db), before);
}
