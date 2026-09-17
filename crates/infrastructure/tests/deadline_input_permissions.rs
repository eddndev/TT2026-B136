mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_input_support;
mod hearing_database_support;
mod hearing_result_database_support;
mod judicial_calendar_database_support;
mod procedural_fact_backend_support;

use application::{deadline_inputs::DeadlineInputStore, ApplicationError};
use deadline_input_support::*;

#[test]
fn even_unknown_source_requires_current_staff_role_and_case_membership() {
    let Some(mut db) = Fixture::new() else { return };
    let reader = store(&db);
    let request = unknown(db.case);
    let mut actors = vec![(db.owner, true, true)];
    for role in ["litigator", "paralegal", "client"] {
        for assigned in [false, true] {
            actors.push((db.user(role, assigned), role != "client", assigned));
        }
    }
    for (actor, staff, assigned) in actors {
        let before = audit(&mut db);
        let result = reader.load(actor, &request);
        if staff && assigned {
            assert!(result.is_ok(), "{result:?}");
            assert_one_read(&mut db, &before);
        } else {
            assert!(
                matches!(result, Err(ApplicationError::CaseNotFound)) && staff
                    || matches!(result, Err(ApplicationError::PermissionDenied)) && !staff,
                "{result:?}"
            );
            assert_eq!(audit(&mut db), before);
        }
    }
}

#[test]
fn an_open_reader_rechecks_revoked_membership_and_disabled_user() {
    let Some(mut db) = Fixture::new() else { return };
    let member = db.user("litigator", true);
    let reader = store(&db);
    let request = unknown(db.case);
    reader.load(member, &request).unwrap();
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
            &[&db.case.as_uuid(), &member.as_uuid()],
        )
        .unwrap();
    let before = audit(&mut db);
    assert!(matches!(
        reader.load(member, &request),
        Err(ApplicationError::CaseNotFound)
    ));
    assert_eq!(audit(&mut db), before);
    db.admin
        .execute(
            "UPDATE users SET active=false WHERE id=$1",
            &[&db.owner.as_uuid()],
        )
        .unwrap();
    assert!(matches!(
        reader.load(db.owner, &request),
        Err(ApplicationError::InvalidSession)
    ));
    assert_eq!(audit(&mut db), before);
}

#[test]
fn a_global_calendar_does_not_bypass_case_scope_for_unknown_sources() {
    let Some(mut db) = Fixture::new() else { return };
    let outsider = db.user("litigator", false);
    let exact = calendar(&db);
    let mut request = unknown(db.case);
    request.calendar = Some(application::deadline_inputs::DeadlineCalendarRef {
        id: exact.id,
        revision: exact.revision,
    });
    let reader = store(&db);
    let before = audit(&mut db);
    assert!(matches!(
        reader.load(outsider, &request),
        Err(ApplicationError::CaseNotFound)
    ));
    assert_eq!(audit(&mut db), before);
    let material = reader.load(db.owner, &request).unwrap();
    assert_eq!(material.calendar, Some(exact));
    assert!(material.source.is_none());
    assert_one_read(&mut db, &before);
}

#[test]
fn closed_case_retains_authorized_reads_and_its_captured_closed_administration() {
    use application::cases::*;
    use domain::identity::Role;
    use procedural_fact_backend_support as facts;
    let Some(mut db) = Fixture::new() else { return };
    let reader = store(&db);
    let paralegal = db.user("paralegal", true);
    let workflow = facts::service(&db, db.owner, Role::Owner);
    let fact = facts::persist(&workflow, db.case, dated_resolution("2026-01-05"));
    let closed = db
        .store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(0),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    let before = audit(&mut db);
    let material = reader.load(paralegal, &fact_request(&fact)).unwrap();
    assert_eq!(material.administration, closed.administration);
    assert_candidate(&fact_request(&fact), &material, "2026-01-06");
    assert_one_read(&mut db, &before);
}

#[test]
fn an_accessible_case_cannot_import_another_cases_resolution_or_notification() {
    use application::{cases::CaseRepository, procedural_facts::*};
    use domain::{
        cases::{CaseId, CaseMetadata},
        identity::Role,
    };
    use procedural_fact_backend_support as facts;
    let Some(mut db) = Fixture::new() else { return };
    let workflow = facts::service(&db, db.owner, Role::Owner);
    let resolution = facts::persist(&workflow, db.case, dated_resolution("2026-01-05"));
    let notification = facts::persist(
        &workflow,
        db.case,
        facts::notify(facts::resolution_ref(&resolution)),
    );
    let other = CaseId::new();
    db.store()
        .create_basic(
            db.owner,
            other,
            CaseMetadata::new("Other case", "REF-OTHER").unwrap(),
            db.at,
        )
        .unwrap();
    let reader = store(&db);
    for source in [resolution, notification] {
        let mut request = fact_request(&source);
        request.trigger.case_id = other;
        let before = audit(&mut db);
        assert!(matches!(
            reader.load(db.owner, &request),
            Err(ApplicationError::ProceduralFact(
                ProceduralFactError::NotFound
            ))
        ));
        assert_eq!(audit(&mut db), before);
    }
}
