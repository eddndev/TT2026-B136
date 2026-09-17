mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_input_history_support;
mod deadline_input_support;
mod hearing_database_support;
mod hearing_result_database_support;
mod judicial_calendar_database_support;
mod procedural_fact_backend_support;

use application::{cases::*, deadline_inputs::*, procedural_facts::*};
use deadline_input_history_support::*;
use deadline_input_support::*;
use domain::hearing_results::HearingResultAgreementId;
use domain::{cases::CaseMetadata, deadline_triggers::*, identity::Role};
use judicial_calendar_database_support as calendars;
use procedural_fact_backend_support as facts;

#[test]
fn captured_resolution_and_calendar_heads_survive_later_retirement() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = facts::service(&db, db.owner, Role::Owner);
    let first = facts::persist(&workflow, db.case, dated_resolution("2026-01-05"));
    let second = facts::persist(&workflow, db.case, correct_resolution(&first, "2026-01-08"));
    let calendar = calendar(&db);
    let calendar_workflow = calendars::service(&db, db.owner, Role::Owner);
    let calendar_head = calendars::persist(&calendar_workflow, calendars::replace(&calendar));
    let mut request = fact_request(&first);
    request.calendar = Some(DeadlineCalendarRef {
        id: calendar.id,
        revision: calendar.revision,
    });
    let captured = store(&db).load(db.owner, &request).unwrap();
    let heads = DeadlineInputHeads::capture(&captured);
    assert_eq!(
        heads.source,
        Some(match fact_request(&second).trigger.source {
            FactDeclaration::Known(value) => value,
            _ => unreachable!(),
        })
    );
    assert_eq!(
        heads.calendar,
        Some(DeadlineCalendarRef {
            id: calendar_head.id,
            revision: calendar_head.revision,
        })
    );
    facts::persist(&workflow, db.case, facts::withdraw(&second));
    calendars::persist(&calendar_workflow, calendars::retire(&calendar_head));
    advance_administration(&db, 0);
    let before = audit(&mut db);
    assert_eq!(
        historical(&db, &request, &captured, &heads).unwrap(),
        captured
    );
    assert_eq!(audit(&mut db), before);
}

#[test]
fn notification_head_preserves_its_own_exact_parent_revision() {
    let Some(db) = Fixture::new() else { return };
    let workflow = facts::service(&db, db.owner, Role::Owner);
    let parent = facts::persist(&workflow, db.case, dated_resolution("2026-01-05"));
    let first = notice(&db, facts::resolution_ref(&parent), None);
    let newer_parent = facts::persist(
        &workflow,
        db.case,
        correct_resolution(&parent, "2026-01-08"),
    );
    let second = notice(&db, facts::resolution_ref(&newer_parent), Some(&first));
    let request = fact_request(&first);
    let captured = store(&db).load(db.owner, &request).unwrap();
    let heads = DeadlineInputHeads::capture(&captured);
    let Some(TriggerSourceRef::Notification { resolution, .. }) = heads.source else {
        panic!("notification head expected")
    };
    assert_eq!(resolution, facts::resolution_ref(&newer_parent));
    facts::persist(&workflow, db.case, facts::withdraw(&second));
    assert_eq!(
        historical(&db, &request, &captured, &heads).unwrap(),
        captured
    );
    let mut forged = heads;
    let Some(TriggerSourceRef::Notification { resolution, .. }) = &mut forged.source else {
        unreachable!()
    };
    *resolution = facts::resolution_ref(&parent);
    assert!(historical(&db, &request, &captured, &forged).is_err());
}

#[test]
fn hearing_head_does_not_reselect_an_agreement_removed_after_the_exact_revision() {
    let Some(mut db) = Fixture::new() else { return };
    let first = hearing_result(&mut db, true);
    let second = corrected_result(&db, &first);
    let reference = FactHearingRef {
        hearing_id: first.snapshot.hearing_id,
        result_id: first.snapshot.id,
        revision: first.snapshot.revision,
        agreement_id: Some(HearingResultAgreementId::from_uuid(uuid::Uuid::nil())),
    };
    let request = request(db.case, TriggerSourceRef::HearingResult(reference));
    let captured = store(&db).load(db.owner, &request).unwrap();
    let heads = DeadlineInputHeads::capture(&captured);
    let Some(TriggerSourceRef::HearingResult(head)) = heads.source else {
        panic!("hearing result head expected")
    };
    assert_eq!(head.revision, second.snapshot.revision);
    assert_eq!(head.agreement_id, None);
    retired_result(&db, &second);
    assert_eq!(
        historical(&db, &request, &captured, &heads).unwrap(),
        captured
    );
    let mut forged = heads;
    let Some(TriggerSourceRef::HearingResult(head)) = &mut forged.source else {
        unreachable!()
    };
    head.agreement_id = reference.agreement_id;
    assert!(historical(&db, &request, &captured, &forged).is_err());
}

#[test]
fn unrevised_capture_is_preserved_after_case_closure_and_rejects_an_invented_baseline() {
    let Some(db) = Fixture::new() else { return };
    let request = unknown(db.case);
    let captured = store(&db).load(db.owner, &request).unwrap();
    assert!(matches!(
        captured.administration,
        CurrentCaseAdministration::Unrevised(_)
    ));
    let heads = DeadlineInputHeads::capture(&captured);
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(0),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    assert_eq!(
        historical(&db, &request, &captured, &heads).unwrap(),
        captured
    );
    let mut forged = captured;
    forged.administration =
        CurrentCaseAdministration::Unrevised(CaseMetadata::new("Invented", "REF-OLD").unwrap());
    assert!(historical(&db, &request, &forged, &heads).is_err());
}

#[test]
fn recorded_capture_is_exact_including_actor_and_allows_historical_closed_state() {
    let Some(db) = Fixture::new() else { return };
    advance_administration(&db, 0);
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(1),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    let request = unknown(db.case);
    let captured = store(&db).load(db.owner, &request).unwrap();
    let heads = DeadlineInputHeads::capture(&captured);
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(2),
            CaseAdministrativeStatus::Active,
            db.at,
        )
        .unwrap();
    assert_eq!(
        historical(&db, &request, &captured, &heads).unwrap(),
        captured
    );
    let mut forged = captured;
    let CurrentCaseAdministration::Recorded(snapshot) = &mut forged.administration else {
        panic!("recorded administration expected")
    };
    snapshot.changed_by.email = "forged@example.test".into();
    assert!(historical(&db, &request, &forged, &heads).is_err());
}

#[test]
fn historical_head_presence_and_identity_must_match_the_selection() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = facts::service(&db, db.owner, Role::Owner);
    let first = facts::persist(&workflow, db.case, dated_resolution("2026-01-05"));
    let other = facts::persist(&workflow, db.case, dated_resolution("2026-01-06"));
    let request = fact_request(&first);
    let captured = store(&db).load(db.owner, &request).unwrap();
    let heads = DeadlineInputHeads::capture(&captured);
    let before = audit(&mut db);
    let missing = DeadlineInputHeads {
        source: None,
        calendar: None,
    };
    assert!(historical(&db, &request, &captured, &missing).is_err());
    let foreign = DeadlineInputHeads {
        source: Some(TriggerSourceRef::Resolution(facts::resolution_ref(&other))),
        calendar: None,
    };
    assert!(historical(&db, &request, &captured, &foreign).is_err());
    assert!(historical(&db, &unknown(db.case), &captured, &heads).is_err());
    let mut wrong_case = request.clone();
    wrong_case.trigger.case_id = domain::cases::CaseId::new();
    assert!(historical(&db, &wrong_case, &captured, &heads).is_err());
    let mut missing_revision = heads;
    let Some(TriggerSourceRef::Resolution(reference)) = &mut missing_revision.source else {
        unreachable!()
    };
    reference.revision = FactRevision::new(99).unwrap();
    assert!(historical(&db, &request, &captured, &missing_revision).is_err());
    assert_eq!(audit(&mut db), before);
}

#[test]
fn calendar_capture_rejects_missing_unselected_and_foreign_heads() {
    let Some(db) = Fixture::new() else { return };
    let first = calendar(&db);
    let other = calendar(&db);
    let mut request = unknown(db.case);
    request.calendar = Some(DeadlineCalendarRef {
        id: first.id,
        revision: first.revision,
    });
    let captured = store(&db).load(db.owner, &request).unwrap();
    let heads = DeadlineInputHeads::capture(&captured);
    let missing = DeadlineInputHeads {
        source: None,
        calendar: None,
    };
    assert!(historical(&db, &request, &captured, &missing).is_err());
    let foreign = DeadlineInputHeads {
        source: None,
        calendar: Some(DeadlineCalendarRef {
            id: other.id,
            revision: other.revision,
        }),
    };
    assert!(historical(&db, &request, &captured, &foreign).is_err());
    request.calendar = None;
    assert!(historical(&db, &request, &captured, &heads).is_err());
}

#[test]
fn a_corrupt_captured_head_receipt_is_rejected_without_audit() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = facts::service(&db, db.owner, Role::Owner);
    let first = facts::persist(&workflow, db.case, dated_resolution("2026-01-05"));
    let second = facts::persist(&workflow, db.case, correct_resolution(&first, "2026-01-08"));
    let request = fact_request(&first);
    let captured = store(&db).load(db.owner, &request).unwrap();
    let heads = DeadlineInputHeads::capture(&captured);
    corrupt_receipt(&mut db, &second);
    let before = audit(&mut db);
    assert!(historical(&db, &request, &captured, &heads).is_err());
    assert_eq!(audit(&mut db), before);
}

#[test]
fn a_case_created_with_revision_one_cannot_be_read_as_an_unrevised_baseline() {
    let Some(db) = Fixture::new() else { return };
    let id = domain::cases::CaseId::new();
    db.store()
        .create_basic(
            db.owner,
            id,
            CaseMetadata::new("Baseline", "REF-OLD").unwrap(),
            db.at,
        )
        .unwrap();
    let request = unknown(id);
    let captured = DeadlineInputMaterial {
        case_id: id,
        administration: CurrentCaseAdministration::Unrevised(
            CaseMetadata::new("Baseline", "REF-OLD").unwrap(),
        ),
        source: None,
        source_head: None,
        calendar: None,
        calendar_head: None,
    };
    let heads = DeadlineInputHeads::capture(&captured);
    assert!(historical(&db, &request, &captured, &heads).is_err());
}

#[test]
fn captured_heads_cannot_precede_selected_revisions() {
    let Some(db) = Fixture::new() else { return };
    let workflow = facts::service(&db, db.owner, Role::Owner);
    let first = facts::persist(&workflow, db.case, dated_resolution("2026-01-05"));
    let second = facts::persist(&workflow, db.case, correct_resolution(&first, "2026-01-08"));
    let request = fact_request(&second);
    let captured = store(&db).load(db.owner, &request).unwrap();
    let mut heads = DeadlineInputHeads::capture(&captured);
    heads.source = Some(TriggerSourceRef::Resolution(facts::resolution_ref(&first)));
    assert!(historical(&db, &request, &captured, &heads).is_err());
}

#[test]
fn captured_administration_rejects_a_different_offset_of_the_same_instant() {
    let Some(mut db) = Fixture::new() else { return };
    advance_administration(&db, 0);
    let request = unknown(db.case);
    let captured = store(&db).load(db.owner, &request).unwrap();
    let heads = DeadlineInputHeads::capture(&captured);
    let mut forged = captured.clone();
    let CurrentCaseAdministration::Recorded(snapshot) = &mut forged.administration else {
        panic!("recorded administration expected")
    };
    let original = snapshot.changed_at;
    snapshot.changed_at = original.to_offset(time::UtcOffset::from_hms(1, 0, 0).unwrap());
    assert_eq!(snapshot.changed_at, original);
    assert_ne!(snapshot.changed_at.offset(), original.offset());
    let before = audit(&mut db);
    assert!(historical(&db, &request, &forged, &heads).is_err());
    assert_eq!(audit(&mut db), before);
}
