mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_dispatch_family_support;
#[allow(dead_code)]
mod deadline_dispatch_support;
mod deadline_input_support;
mod deadline_profile_database_support;
mod hearing_database_support;
mod hearing_result_database_support;
mod judicial_calendar_database_support;
mod procedural_fact_backend_support;

use application::{
    cases::*, deadline_inputs::DeadlineCalendarRef, deadline_profiles::*,
    deadline_reevaluation::DependencyFamily, deadlines::*, procedural_facts::*,
};
use deadline_backend_support as dl;
use deadline_dispatch_family_support as family;
use deadline_dispatch_support as dispatch;
use deadline_input_support as inputs;
use deadline_profile_database_support as profiles;
use domain::{deadline_triggers::*, identity::Role};
use judicial_calendar_database_support as calendars;
use procedural_fact_backend_support as facts;
use uuid::Uuid;

#[test]
fn global_and_case_profiles_and_calendar_select_current_heads_even_after_closure_and_revocation() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    case_stage_database_support::complete(&db);
    let first_case = db.case;
    let responsible = db.user("paralegal", true);
    let global = family::publish(&db, None, TriggerField::ResolutionIssuedAt);
    let local = family::publish(&db, Some(first_case), TriggerField::ResolutionIssuedAt);
    let shared_calendar = inputs::calendar(&db);
    let other_calendar = inputs::calendar(&db);
    let source = dl::source(&db);
    for (id, profile, calendar) in [
        (10, &global, &shared_calendar),
        (20, &local, &other_calendar),
        (30, &global, &shared_calendar),
    ] {
        let mut command = dispatch::command(&db, profile, &source, id);
        let definition = dl::definition_mut(&mut command);
        definition.input.calendar = Some(DeadlineCalendarRef {
            id: calendar.id,
            revision: calendar.revision,
        });
        if id == 10 {
            definition.responsible = responsible;
        }
        let value = family::persist(&db, command);
        if id == 30 {
            let mut command = dl::correct(&value);
            let definition = dl::definition_mut(&mut command);
            definition.profile = DeadlineProfileRef {
                id: local.id,
                revision: local.revision,
            };
            definition.input.calendar = Some(DeadlineCalendarRef {
                id: other_calendar.id,
                revision: other_calendar.revision,
            });
            family::persist(&db, command);
        }
    }
    let second_case = family::new_case(&db);
    db.case = second_case;
    let second_local = family::publish(&db, Some(second_case), TriggerField::ResolutionIssuedAt);
    let second_source = dl::source(&db);
    for (id, profile, calendar) in [
        (40, &global, &shared_calendar),
        (50, &second_local, &other_calendar),
    ] {
        let mut command = dispatch::command(&db, profile, &second_source, id);
        dl::definition_mut(&mut command).input.calendar = Some(DeadlineCalendarRef {
            id: calendar.id,
            revision: calendar.revision,
        });
        family::persist(&db, command);
    }
    db.case = first_case;
    let store = dispatch::open(&db);
    dispatch::drain_existing(&mut db, &store);
    let workflow = profiles::service(&db, db.owner, Role::Owner);
    let global_head = profiles::persist(
        &workflow,
        DeadlineProfileCollection::Global,
        profiles::replace(&global),
    );
    let local_head = profiles::persist(
        &workflow,
        DeadlineProfileCollection::ForCase(first_case),
        profiles::replace(&local),
    );
    let second_head = profiles::persist(
        &workflow,
        DeadlineProfileCollection::ForCase(second_case),
        profiles::replace(&second_local),
    );
    let calendar_head = calendars::persist(
        &calendars::service(&db, db.owner, Role::Owner),
        calendars::replace(&shared_calendar),
    );
    db.store()
        .change_administrative_status(
            db.owner,
            first_case,
            CaseRevisionExpectation::new(1),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    db.admin
        .execute(
            "UPDATE users SET active=FALSE WHERE id=$1",
            &[&responsible.as_uuid()],
        )
        .unwrap();
    db.admin
        .execute(
            "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
            &[&first_case.as_uuid(), &responsible.as_uuid()],
        )
        .unwrap();
    let history = dispatch::deadline_history(&mut db);
    let global_event = family::assert_event(
        &mut db,
        &store,
        global_head.receipt.operation_id.as_uuid(),
        &[10, 40],
    );
    assert_eq!(global_event.family, DependencyFamily::Profile);
    assert!(global_event.case_id.is_none());
    let local_event = family::assert_event(
        &mut db,
        &store,
        local_head.receipt.operation_id.as_uuid(),
        &[20, 30],
    );
    assert_eq!(local_event.case_id, Some(first_case));
    let second_event = family::assert_event(
        &mut db,
        &store,
        second_head.receipt.operation_id.as_uuid(),
        &[50],
    );
    assert_eq!(second_event.case_id, Some(second_case));
    let calendar_event = family::assert_event(
        &mut db,
        &store,
        calendar_head.receipt.operation_id.as_uuid(),
        &[10, 40],
    );
    assert_eq!(calendar_event.family, DependencyFamily::Calendar);
    assert!(calendar_event.case_id.is_none());
    assert!(calendar_event.hearing_id.is_none());
    assert_eq!(dispatch::deadline_history(&mut db), history);
}

#[test]
fn resolution_parent_and_notification_events_keep_family_identity_and_ignore_old_head_matches() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let parent = dl::source(&db);
    let other_parent = dl::source(&db);
    let same_uuid = facts::resolution_ref(&parent).id.as_uuid();
    let notice = family::notice(&db, &parent, same_uuid);
    let other_notice = family::notice(&db, &other_parent, Uuid::new_v4());
    let direct = family::publish(&db, Some(db.case), TriggerField::ResolutionIssuedAt);
    let notification = family::publish(&db, Some(db.case), TriggerField::NotificationPracticedAt);
    dispatch::legacy(&db, &direct, &parent, 10);
    for (id, selected) in [(20, &notice), (30, &other_notice), (40, &notice)] {
        let mut command = dispatch::command(&db, &notification, &parent, id);
        dl::definition_mut(&mut command).input.selection = inputs::fact_request(selected).trigger;
        let value = family::persist(&db, command);
        if id == 40 {
            let mut command = dl::correct(&value);
            dl::definition_mut(&mut command).input.selection =
                inputs::fact_request(&other_notice).trigger;
            family::persist(&db, command);
        }
    }
    let store = dispatch::open(&db);
    dispatch::drain_existing(&mut db, &store);
    let workflow = facts::service(&db, db.owner, Role::Owner);
    let parent_head = facts::persist(&workflow, db.case, facts::correct(&parent));
    let notice_head = facts::persist(&workflow, db.case, facts::correct(&notice));
    let other_head = facts::persist(&workflow, db.case, facts::correct(&other_parent));
    let history = dispatch::deadline_history(&mut db);
    let event = family::assert_event(
        &mut db,
        &store,
        parent_head
            .snapshot
            .metadata()
            .receipt
            .operation_id
            .as_uuid(),
        &[10, 20],
    );
    assert_eq!(event.family, DependencyFamily::Resolution);
    assert_eq!(event.source_id, same_uuid);
    assert_eq!(event.case_id, Some(db.case));
    let event = family::assert_event(
        &mut db,
        &store,
        notice_head
            .snapshot
            .metadata()
            .receipt
            .operation_id
            .as_uuid(),
        &[20],
    );
    assert_eq!(event.family, DependencyFamily::Notification);
    assert_eq!(event.source_id, same_uuid);
    family::assert_event(
        &mut db,
        &store,
        other_head
            .snapshot
            .metadata()
            .receipt
            .operation_id
            .as_uuid(),
        &[30, 40],
    );
    assert_eq!(dispatch::deadline_history(&mut db), history);
    let value = dl::store(&db)
        .get(db.owner, db.case, dispatch::id(20), None, db.at)
        .unwrap();
    let FactDeclaration::Known(TriggerSourceRef::Notification { resolution, .. }) =
        value.definition.input.selection.source
    else {
        panic!("notification selection expected")
    };
    assert_eq!(resolution, facts::resolution_ref(&parent));
}

#[test]
fn hearing_result_events_bind_the_hearing_and_current_selected_source_with_cross_family_uuid_collision(
) {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let first = inputs::hearing_result(&mut db, true);
    let second = family::result_in_current_case(&db);
    assert_ne!(first.snapshot.hearing_id, second.snapshot.hearing_id);
    let source = family::resolution(&db, first.snapshot.id.as_uuid());
    let profile = family::publish(&db, Some(db.case), TriggerField::HearingSessionEventTime);
    let resolution_profile = family::publish(&db, Some(db.case), TriggerField::ResolutionIssuedAt);
    dispatch::legacy(&db, &resolution_profile, &source, 40);
    let reference = |value: &application::hearing_results::HearingResultDetail| {
        TriggerSourceRef::HearingResult(FactHearingRef {
            hearing_id: value.snapshot.hearing_id,
            result_id: value.snapshot.id,
            revision: value.snapshot.revision,
            agreement_id: Some(
                domain::hearing_results::HearingResultAgreementId::from_uuid(Uuid::nil()),
            ),
        })
    };
    for (id, selected) in [(10, &first), (20, &second), (30, &first)] {
        let mut command = dispatch::command(&db, &profile, &source, id);
        dl::definition_mut(&mut command).input.selection =
            inputs::request(db.case, reference(selected)).trigger;
        let value = family::persist(&db, command);
        if id == 30 {
            let mut command = dl::correct(&value);
            dl::definition_mut(&mut command).input.selection =
                inputs::request(db.case, reference(&second)).trigger;
            family::persist(&db, command);
        }
    }
    let store = dispatch::open(&db);
    dispatch::drain_existing(&mut db, &store);
    let first_head = inputs::corrected_result(&db, &first);
    let second_head = inputs::corrected_result(&db, &second);
    let history = dispatch::deadline_history(&mut db);
    let event = family::assert_event(
        &mut db,
        &store,
        first_head.snapshot.receipt.operation_id.as_uuid(),
        &[10],
    );
    assert_eq!(event.family, DependencyFamily::HearingResult);
    assert_eq!(event.source_id, first.snapshot.id.as_uuid());
    assert_eq!(event.case_id, Some(db.case));
    assert_eq!(event.hearing_id, Some(first.snapshot.hearing_id.as_uuid()));
    let event = family::assert_event(
        &mut db,
        &store,
        second_head.snapshot.receipt.operation_id.as_uuid(),
        &[20, 30],
    );
    assert_eq!(event.hearing_id, Some(second.snapshot.hearing_id.as_uuid()));
    assert_eq!(dispatch::deadline_history(&mut db), history);
}
