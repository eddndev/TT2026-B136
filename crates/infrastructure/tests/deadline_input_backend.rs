mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_input_support;
mod hearing_database_support;
#[path = "deadline_input_support/hearings.rs"]
mod hearing_inputs;
mod hearing_result_database_support;
mod judicial_calendar_database_support;
mod procedural_fact_backend_support;

use application::{cases::CurrentCaseAdministration, deadline_inputs::*};
use deadline_input_support::*;
use domain::identity::Role;
use procedural_fact_backend_support as facts;

#[test]
fn unknown_source_preserves_real_unrevised_case_and_appends_one_read() {
    let Some(mut db) = Fixture::new() else { return };
    let reader = store(&db);
    let before = audit(&mut db);
    let material = reader.load(db.owner, &unknown(db.case)).unwrap();
    assert_eq!(material.case_id, db.case);
    let CurrentCaseAdministration::Unrevised(metadata) = material.administration else {
        panic!("unrevised case expected")
    };
    assert_eq!(
        (metadata.title(), metadata.reference()),
        ("Baseline", "REF-OLD")
    );
    assert!(material.source.is_none());
    assert!(material.source_head.is_none());
    assert!(material.calendar.is_none());
    assert!(material.calendar_head.is_none());
    assert_one_read(&mut db, &before);
}

#[test]
fn resolution_exact_revision_and_current_withdrawn_head_remain_distinct() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = facts::service(&db, db.owner, Role::Owner);
    let first = facts::persist(&workflow, db.case, dated_resolution("2026-01-05"));
    let corrected = facts::persist(&workflow, db.case, correct_resolution(&first, "2026-01-09"));
    let head = facts::persist(&workflow, db.case, facts::withdraw(&corrected));
    let reader = store(&db);
    let before = audit(&mut db);
    let request = fact_request(&first);
    let material = reader.load(db.owner, &request).unwrap();
    assert_candidate(&request, &material, "2026-01-06");
    assert_eq!(
        material.source,
        Some(DeadlineSourceDetail::Fact(Box::new(first)))
    );
    assert_eq!(
        material.source_head,
        Some(DeadlineSourceDetail::Fact(Box::new(head.clone())))
    );
    assert_one_read(&mut db, &before);
    let before = audit(&mut db);
    let request = fact_request(&head);
    let material = reader.load(db.owner, &request).unwrap();
    assert_eq!(material.source, material.source_head);
    assert_candidate(&request, &material, "2026-01-10");
    assert_one_read(&mut db, &before);
}

#[test]
fn notification_preserves_its_exact_parent_revision_when_current_head_selects_a_newer_one() {
    use application::procedural_facts::*;
    let Some(mut db) = Fixture::new() else { return };
    let workflow = facts::service(&db, db.owner, Role::Owner);
    let parent = facts::persist(&workflow, db.case, dated_resolution("2026-01-05"));
    let old_parent = facts::resolution_ref(&parent);
    let id = NotificationId::new();
    let first = facts::persist(
        &workflow,
        db.case,
        ProceduralFactCommand::Notification(
            NotificationCommand::new(
                FactOperationId::new(),
                id,
                old_parent.id,
                FactChange::record(notification_at(old_parent, "2026-01-06")),
            )
            .unwrap(),
        ),
    );
    let parent_head = facts::persist(
        &workflow,
        db.case,
        correct_resolution(&parent, "2026-01-08"),
    );
    let new_parent = facts::resolution_ref(&parent_head);
    let corrected = facts::persist(
        &workflow,
        db.case,
        ProceduralFactCommand::Notification(
            NotificationCommand::new(
                FactOperationId::new(),
                id,
                new_parent.id,
                FactChange::correct(
                    first.snapshot.metadata().revision,
                    notification_at(new_parent, "2026-01-09"),
                    facts::text("Correct parent and date"),
                ),
            )
            .unwrap(),
        ),
    );
    let head = facts::persist(&workflow, db.case, facts::withdraw(&corrected));
    let reader = store(&db);
    let request = fact_request(&first);
    let before = audit(&mut db);
    let material = reader.load(db.owner, &request).unwrap();
    assert_candidate(&request, &material, "2026-01-07");
    assert_eq!(
        material.source,
        Some(DeadlineSourceDetail::Fact(Box::new(first)))
    );
    assert_eq!(
        material.source_head,
        Some(DeadlineSourceDetail::Fact(Box::new(head)))
    );
    assert_one_read(&mut db, &before);
}

#[test]
fn calendar_exact_revision_drives_arithmetic_after_replacement_and_retirement() {
    use domain::deadline_arithmetic::{ArithmeticRule, DayBasis, DayInclusion, FinalDayPolicy};
    use judicial_calendar_database_support as calendars;
    let Some(mut db) = Fixture::new() else { return };
    let workflow = facts::service(&db, db.owner, Role::Owner);
    let fact = facts::persist(&workflow, db.case, dated_resolution("2026-01-05"));
    let exact = calendar(&db);
    let calendars = calendars::service(&db, db.owner, Role::Owner);
    let changed = calendars::persist(&calendars, calendars::replace(&exact));
    let head = calendars::persist(&calendars, calendars::retire(&changed));
    let mut request = fact_request(&fact);
    request.calendar = Some(DeadlineCalendarRef {
        id: exact.id,
        revision: exact.revision,
    });
    request.rule = ArithmeticRule::Days {
        quantity: std::num::NonZeroU32::new(1).unwrap(),
        inclusion: DayInclusion::AfterAnchor,
        basis: DayBasis::CalendarCountable,
        final_day: FinalDayPolicy::Preserve,
    };
    let reader = store(&db);
    let before = audit(&mut db);
    let material = reader.load(db.owner, &request).unwrap();
    assert_candidate(&request, &material, "2026-01-06");
    assert_eq!(material.calendar, Some(exact));
    assert_eq!(material.calendar_head, Some(head));
    assert_one_read(&mut db, &before);
}

#[test]
fn corrupt_current_receipt_rejects_an_otherwise_valid_exact_source_without_audit() {
    use application::{procedural_facts::ProceduralFactError, ApplicationError};
    let Some(mut db) = Fixture::new() else { return };
    let workflow = facts::service(&db, db.owner, Role::Owner);
    let first = facts::persist(&workflow, db.case, dated_resolution("2026-01-05"));
    let head = facts::persist(&workflow, db.case, correct_resolution(&first, "2026-01-09"));
    let reader = store(&db);
    let id = facts::resolution_ref(&head).id;
    db.admin
        .batch_execute(
            "ALTER TABLE case_procedural_fact_revisions DISABLE TRIGGER USER;
        ALTER TABLE case_procedural_fact_revisions DROP CONSTRAINT procedural_fact_submission_hash",
        )
        .unwrap();
    db.admin
        .execute(
            "UPDATE case_procedural_fact_revisions SET submission_digest=$3
        WHERE family='resolution' AND id=$1 AND revision=$2",
            &[
                &id.as_uuid(),
                &i64::from(head.snapshot.metadata().revision.get()),
                &vec![0_u8; 32],
            ],
        )
        .unwrap();
    let before = audit(&mut db);
    assert!(matches!(
        reader.load(db.owner, &fact_request(&first)),
        Err(ApplicationError::ProceduralFact(
            ProceduralFactError::StoredInconsistent(_)
        ))
    ));
    assert_eq!(audit(&mut db), before);
}

#[test]
fn unknown_source_does_not_hide_a_missing_selected_calendar() {
    use application::{judicial_calendars::*, ApplicationError};
    let Some(mut db) = Fixture::new() else { return };
    let mut request = unknown(db.case);
    request.calendar = Some(DeadlineCalendarRef {
        id: JudicialCalendarId::new(),
        revision: JudicialCalendarRevision::initial(),
    });
    let reader = store(&db);
    let before = audit(&mut db);
    assert!(matches!(
        reader.load(db.owner, &request),
        Err(ApplicationError::JudicialCalendar(
            JudicialCalendarError::NotFound
        ))
    ));
    assert_eq!(audit(&mut db), before);
}
