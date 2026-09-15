mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;

use application::case_stages::*;
use application::cases::*;
use application::ApplicationError;
use case_stage_database_support::*;
use domain::identity::Role;

#[test]
fn four_roles_and_case_membership_are_rechecked_for_reads_and_writes() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&db);
    let record = upload(&db, db.case, "support.pdf");
    let stages = store(&db);
    let change = CaseStageChange::Adopt(adoption(&db, &record, CaseStage::Investigation));
    for (role, read, write) in [
        ("owner", true, true),
        ("litigator", true, true),
        ("paralegal", true, false),
        ("client", false, false),
    ] {
        let actor = db.user(role, true);
        assert_eq!(stages.get(actor, db.case, db.at).is_ok(), read, "{role}");
        assert_eq!(
            stages
                .history(
                    actor,
                    db.case,
                    CaseStageQuery::new(10, None).unwrap(),
                    db.at
                )
                .is_ok(),
            read,
            "{role}"
        );
        let before = snapshot(&mut db);
        assert_eq!(
            stages
                .prepare(
                    actor,
                    db.case,
                    CaseStageExpectation::Unregistered,
                    &change,
                    &StageSupportReadLimits::standard()
                )
                .is_ok(),
            write,
            "{role}"
        );
        assert_eq!(snapshot(&mut db), before);
    }
    let unassigned = db.user("litigator", false);
    assert!(matches!(
        stages.get(unassigned, db.case, db.at),
        Err(ApplicationError::CaseNotFound)
    ));
    assert!(matches!(
        stages.prepare(
            unassigned,
            db.case,
            CaseStageExpectation::Unregistered,
            &change,
            &StageSupportReadLimits::standard()
        ),
        Err(ApplicationError::CaseNotFound)
    ));
}

#[test]
fn revocation_role_change_and_disabled_identity_after_preparation_cannot_commit() {
    for mutation in [
        "DELETE FROM case_memberships WHERE user_id=$1",
        "UPDATE users SET active=FALSE WHERE id=$1",
        "UPDATE users SET role='paralegal' WHERE id=$1",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        complete(&db);
        let actor = db.user("litigator", true);
        let record = upload(&db, db.case, "support.pdf");
        let before = snapshot(&mut db);
        let url = db.admin_url.clone();
        let format = FormatCheck(Some(Box::new(move || {
            postgres::Client::connect(&url, postgres::NoTls)
                .unwrap()
                .execute(mutation, &[&actor.as_uuid()])
                .unwrap();
        })));
        let result = service(&db, actor, Role::Litigator, format).adopt(
            "session",
            db.case,
            CaseStageExpectation::Unregistered,
            adoption(&db, &record, CaseStage::Investigation),
        );
        assert!(
            matches!(
                result,
                Err(ApplicationError::CaseNotFound
                    | ApplicationError::InvalidSession
                    | ApplicationError::PermissionDenied)
            ),
            "{result:?}"
        );
        assert_eq!(snapshot(&mut db), before);
    }
}

#[test]
fn concurrent_closure_blocks_commit_and_history_remains_readable() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&db);
    let record = upload(&db, db.case, "support.pdf");
    let repository = db.store();
    let (owner, case, at) = (db.owner, db.case, db.at);
    let format = FormatCheck(Some(Box::new(move || {
        repository
            .change_administrative_status(
                owner,
                case,
                CaseRevisionExpectation::new(1),
                CaseAdministrativeStatus::Closed,
                at,
            )
            .unwrap();
    })));
    let result = service(&db, db.owner, Role::Owner, format).adopt(
        "session",
        db.case,
        CaseStageExpectation::Unregistered,
        adoption(&db, &record, CaseStage::Investigation),
    );
    assert!(matches!(result, Err(ApplicationError::CaseClosed)));
    assert_eq!(
        store(&db).get(db.owner, db.case, db.at).unwrap().current,
        CurrentCaseStage::Unregistered
    );
    assert!(store(&db)
        .history(
            db.owner,
            db.case,
            CaseStageQuery::new(10, None).unwrap(),
            db.at
        )
        .unwrap()
        .entries
        .is_empty());
    let count: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM audit_events WHERE action='case.stage_adopted'",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(count, 0);
}

#[test]
fn runtime_cannot_update_delete_truncate_or_take_ownership_of_stage_history() {
    let Some(mut db) = Fixture::new() else { return };
    let mut runtime = db.runtime();
    for sql in [
        "UPDATE case_stage_revisions SET reason='changed'",
        "DELETE FROM case_stage_revisions",
        "TRUNCATE case_stage_revisions",
        "ALTER TABLE case_stage_revisions DISABLE TRIGGER case_stage_sequence",
    ] {
        assert_eq!(
            runtime
                .batch_execute(sql)
                .unwrap_err()
                .code()
                .unwrap()
                .code(),
            "42501"
        );
    }
    db.admin
        .batch_execute(&format!(
            "GRANT UPDATE ON case_stage_revisions TO {}",
            db.role
        ))
        .unwrap();
    assert!(matches!(
        infrastructure::PostgresCaseStageStore::open(
            &db.runtime_url,
            std::sync::Arc::new(infrastructure::RingSha256Hasher)
        ),
        Err(ApplicationError::InvalidConfiguration(_))
    ));
}
