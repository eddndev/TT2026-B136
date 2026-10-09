use crate::case_report_support::*;
use application::{
    case_reports::*,
    cases::{CaseRepository, CaseRevisionExpectation},
    ApplicationError,
};
use domain::{
    case_administration::{CaseAdministrativeStatus, CaseRevision},
    identity::UserId,
};
use time::Duration;

fn query() -> CaseReportLitigatorQuery {
    CaseReportLitigatorQuery {
        kind: CaseReportKind::CaseState,
        limit: 100,
        after_id: None,
    }
}

#[test]
fn litigator_picker_includes_closed_only_colleague_and_loses_revoked_membership() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    let closed = seed_case(&mut db, "Closed shared case", at);
    db.case = closed;
    let actor_id = db.user("litigator", true);
    let colleague_id = db.user("litigator", true);
    let inactive = db.user("litigator", true);
    db.user("paralegal", true);
    db.user("client", true);
    let foreign = db.user("litigator", false);
    db.admin.execute("UPDATE users SET active=false,revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1", &[&inactive.as_uuid()]).unwrap();
    let actor = principal(&mut db, actor_id);
    let repository = db.store();
    repository
        .change_administrative_status(
            db.owner,
            closed,
            CaseRevisionExpectation::Revision(CaseRevision::FIRST),
            CaseAdministrativeStatus::Closed,
            at,
        )
        .unwrap();
    let store = store(&db, clock(at));
    let page = store.litigators(&actor, query(), at).unwrap();
    let mut expected = vec![actor_id, colleague_id];
    expected.sort_by_key(|id| id.as_uuid());
    assert_eq!(page.scope, CaseReportScope::AssignedCases);
    assert_eq!(page.checked_at, at);
    assert_eq!(
        page.litigators
            .iter()
            .map(|v| v.user_id)
            .collect::<Vec<_>>(),
        expected
    );
    assert!(!page.has_more);
    assert_eq!(page.next_after_id, None);
    assert!(!page.litigators.iter().any(|v| v.user_id == foreign));
    let mut selected = command(at);
    selected.filters.litigator = Some(colleague_id);
    request(&store, &actor, selected.clone(), at).unwrap();
    repository
        .remove_member(closed, actor_id, db.owner, at)
        .unwrap();
    assert!(store
        .litigators(&actor, query(), at)
        .unwrap()
        .litigators
        .is_empty());
    selected.operation_id = CaseReportOperationId::new();
    assert!(matches!(
        request(&store, &actor, selected, at),
        Err(ApplicationError::InvalidInput(_))
    ));
    assert_eq!(audits(&mut db, "case_report.read"), 2);
}

#[test]
fn litigator_picker_owner_paginates_all_active_litigators_including_zero_cases() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    let owner = owner(&mut db);
    let mut expected: Vec<_> = (0..5).map(|_| db.user("litigator", false)).collect();
    expected.sort_by_key(|id| id.as_uuid());
    let inactive = db.user("litigator", false);
    db.admin.execute("UPDATE users SET active=false,revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1", &[&inactive.as_uuid()]).unwrap();
    db.user("paralegal", false);
    let store = store(&db, clock(at));
    let mut result = Vec::new();
    let mut after_id = None;
    for index in 0..3 {
        let page = store
            .litigators(
                &owner,
                CaseReportLitigatorQuery {
                    kind: CaseReportKind::CaseState,
                    limit: 2,
                    after_id,
                },
                at,
            )
            .unwrap();
        assert_eq!(page.scope, CaseReportScope::Office);
        assert_eq!(page.checked_at, at);
        assert_eq!(page.has_more, index < 2);
        assert_eq!(
            page.next_after_id,
            if page.has_more {
                page.litigators.last().map(|v| v.user_id)
            } else {
                None
            }
        );
        after_id = page.next_after_id;
        for value in page.litigators {
            assert_eq!(value.email, format!("{}@example.test", value.user_id));
            result.push(value.user_id);
        }
    }
    assert_eq!(result, expected);
    assert_eq!(audits(&mut db, "case_report.read"), 3);
}

#[test]
fn litigator_picker_reauthenticates_even_empty_pages_and_denies_other_roles() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    let empty = CaseReportLitigatorQuery {
        kind: CaseReportKind::CaseState,
        limit: 2,
        after_id: Some(UserId::from_uuid(uuid::Uuid::from_u128(u128::MAX))),
    };
    let store = store(&db, clock(at));
    for role in ["client", "paralegal"] {
        let id = db.user(role, false);
        let actor = principal(&mut db, id);
        assert!(matches!(
            store.litigators(&actor, empty, at),
            Err(ApplicationError::PermissionDenied)
        ));
    }
    let mut stale = owner(&mut db);
    stale.email = "stale@example.test".into();
    assert!(matches!(
        store.litigators(&stale, empty, at),
        Err(ApplicationError::InvalidSession)
    ));
    let id = db.user("litigator", false);
    let disabled = principal(&mut db, id);
    db.admin.execute("UPDATE users SET active=false,revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1", &[&id.as_uuid()]).unwrap();
    assert!(matches!(
        store.litigators(&disabled, empty, at),
        Err(ApplicationError::InvalidSession)
    ));
    assert_eq!(audits(&mut db, "case_report.read"), 0);
}

#[test]
fn litigator_picker_invalid_cursor_limits_and_clock_write_no_audit() {
    let Some(mut db) = fixture() else { return };
    let at = db.at;
    let owner = owner(&mut db);
    let timer = clock(at);
    let store = store(&db, timer.clone());
    for query in [
        CaseReportLitigatorQuery {
            kind: CaseReportKind::CaseState,
            limit: 0,
            after_id: None,
        },
        CaseReportLitigatorQuery {
            kind: CaseReportKind::CaseState,
            limit: 101,
            after_id: None,
        },
        CaseReportLitigatorQuery {
            kind: CaseReportKind::CaseState,
            limit: 1,
            after_id: Some(UserId::from_uuid(uuid::Uuid::nil())),
        },
    ] {
        assert!(matches!(
            store.litigators(&owner, query, at),
            Err(ApplicationError::InvalidInput(_))
        ));
    }
    timer.set(at - Duration::seconds(1));
    assert!(matches!(
        store.litigators(&owner, query(), at),
        Err(ApplicationError::CaseReport(
            CaseReportError::StoredInconsistent(_)
        ))
    ));
    assert_eq!(audits(&mut db, "case_report.read"), 0);
}
