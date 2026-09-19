mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
#[allow(dead_code)]
mod deadline_dispatch_family_support;
#[allow(dead_code)]
mod deadline_dispatch_support;
mod deadline_input_support;
mod deadline_profile_database_support;
mod deadline_tracked_backend_support;
#[allow(dead_code)]
mod deadline_worker_backend_support;
mod deadline_worker_extra_support;
mod hearing_database_support;
mod hearing_result_database_support;
mod judicial_calendar_database_support;
mod procedural_fact_backend_support;
#[path = "deadline_worker_extra_support/calendar.rs"]
mod worker_calendar;

use application::{
    cases::*,
    deadline_inputs::DeadlineCalendarRef,
    deadline_profiles::*,
    deadline_reevaluation::{DependencyFamily, ObservationRole},
    deadline_tracking::*,
    procedural_facts::*,
};
use deadline_backend_support as dl;
use deadline_dispatch_family_support as family;
use deadline_dispatch_support as dispatch;
use deadline_input_support as inputs;
use deadline_profile_database_support as profiles;
use deadline_tracked_backend_support as tracked;
use deadline_worker_backend_support as worker;
use deadline_worker_extra_support as extra;
use domain::{deadline_triggers::*, hearing_results::HearingResultAgreementId, identity::Role};
use judicial_calendar_database_support as calendars;
use procedural_fact_backend_support as facts;
use time::Duration;
use uuid::Uuid;

#[test]
fn followed_calendar_recalculates_the_due_date_and_preserves_the_old_capture() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let calendar = inputs::calendar(&db);
    let profile = worker_calendar::profile(&db, &calendar);
    let source = dl::source(&db);
    extra::clear_events(&mut db);
    let mut command = dispatch::command(&db, &profile, &source, 810);
    dl::definition_mut(&mut command).input.calendar = Some(DeadlineCalendarRef {
        id: calendar.id,
        revision: calendar.revision,
    });
    let mut policies = tracked::policies();
    policies.calendar = TrackingPolicy::Follow;
    let first = extra::commit(&db, &command, Some(policies), None);
    let base = extra::commit(&db, &dl::attention(&first), None, None);
    let old_row = tracked::revision_row(&mut db, &base);
    let old_due = base.operational_due_at().unwrap();
    assert_eq!(
        old_due.date(),
        time::Date::from_calendar_date(2026, time::Month::January, 7).unwrap()
    );
    let changed = worker_calendar::exclude_january_seventh(&db, &calendar);
    let (job, event) = extra::event_job(&mut db, changed.receipt.operation_id.as_uuid());
    assert_eq!(event.family, DependencyFamily::Calendar);
    let (next, result) = extra::revision(&mut db, &base, &job, event);
    assert_eq!(next.review_state(), DeadlineReviewState::Accepted);
    assert_eq!(
        next.operational_due_at().unwrap() - old_due,
        Duration::days(1)
    );
    assert_eq!(
        next.definition.input.calendar.unwrap().revision,
        changed.revision
    );
    assert_eq!(next.calculation.material.calendar, Some(changed.clone()));
    assert_eq!(
        next.calculation.material.calendar_head,
        Some(changed.clone())
    );
    assert_eq!(next.definition.profile, base.definition.profile);
    assert_eq!(
        next.calculation.material.source,
        base.calculation.material.source
    );
    assert_eq!(next.responsible, base.responsible);
    assert_eq!(next.attention, base.attention);
    assert_eq!(tracked::revision_row(&mut db, &base), old_row);
    calendars::persist(
        &calendars::service(&db, db.owner, Role::Owner),
        calendars::retire(&changed),
    );
    extra::history(&mut db, &[first, base, next], &[result]);
}

#[test]
fn global_and_case_profile_events_preserve_fixed_choices_and_require_review_after_retirement() {
    for (global, fixed, retired) in [
        (true, false, false),
        (false, true, false),
        (false, true, true),
    ] {
        let Some(mut db) = dl::Fixture::new() else {
            return;
        };
        case_stage_database_support::complete(&db);
        let responsible = db.user("paralegal", true);
        let scope = (!global).then_some(db.case);
        let profile = family::publish(&db, scope, TriggerField::ResolutionIssuedAt);
        let source = dl::source(&db);
        extra::clear_events(&mut db);
        let mut command = dispatch::command(&db, &profile, &source, 811);
        dl::definition_mut(&mut command).responsible = responsible;
        let mut policies = tracked::policies();
        policies.profile = if fixed {
            TrackingPolicy::Fixed
        } else {
            TrackingPolicy::Follow
        };
        let base = extra::commit(&db, &command, Some(policies), None);
        let collection = scope.map_or(
            DeadlineProfileCollection::Global,
            DeadlineProfileCollection::ForCase,
        );
        let change = if retired {
            profiles::retire(&profile)
        } else {
            profiles::replace(&profile)
        };
        let head = profiles::persist(
            &profiles::service(&db, db.owner, Role::Owner),
            collection,
            change,
        );
        db.store()
            .change_administrative_status(
                db.owner,
                db.case,
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
                &[&db.case.as_uuid(), &responsible.as_uuid()],
            )
            .unwrap();
        let (job, event) = extra::event_job(&mut db, head.receipt.operation_id.as_uuid());
        assert_eq!(event.family, DependencyFamily::Profile);
        assert_eq!(event.case_id, scope);
        let (next, result) = extra::revision(&mut db, &base, &job, event);
        worker::assert_preserved(&base, &next);
        assert_eq!(next.tracking.as_ref().unwrap().policies, policies);
        assert_eq!(
            extra::observed(&next, ObservationRole::Profile).revision,
            head.revision.get()
        );
        if fixed && !retired {
            assert_eq!(next.review_state(), DeadlineReviewState::Accepted);
            assert!(next.tracking.as_ref().unwrap().review.reasons().is_empty());
        } else {
            assert_eq!(next.review_state(), DeadlineReviewState::Pending);
            let reason = if retired {
                TrackingReviewReason::DependencyRetired
            } else {
                TrackingReviewReason::ProfileChanged
            };
            assert!(next
                .tracking
                .as_ref()
                .unwrap()
                .review
                .reasons()
                .iter()
                .any(|r| r.dependency == TrackingDependency::Profile && r.reason == reason));
        }
        extra::history(&mut db, &[base, next], &[result]);
    }
}

#[test]
fn notification_and_parent_events_with_the_same_uuid_keep_the_exact_selected_parent() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let parent = dl::source(&db);
    let parent_ref = facts::resolution_ref(&parent);
    let notice = family::notice(&db, &parent, parent_ref.id.as_uuid());
    let profile = family::publish(&db, Some(db.case), TriggerField::NotificationPracticedAt);
    extra::clear_events(&mut db);
    let mut command = dispatch::command(&db, &profile, &parent, 812);
    dl::definition_mut(&mut command).input.selection = inputs::fact_request(&notice).trigger;
    let base = extra::commit(&db, &command, Some(tracked::policies()), Some(&parent));
    let parent_head = dispatch::advance(&db, &parent);
    let (parent_job, parent_event) = extra::event_job(
        &mut db,
        parent_head
            .snapshot
            .metadata()
            .receipt
            .operation_id
            .as_uuid(),
    );
    let (after_parent, first_result) = extra::revision(&mut db, &base, &parent_job, parent_event);
    assert_eq!(parent_event.family, DependencyFamily::Resolution);
    assert_eq!(
        extra::observed(&after_parent, ObservationRole::Source).revision,
        1
    );
    assert_eq!(
        extra::observed(&after_parent, ObservationRole::NotificationParent).revision,
        2
    );
    worker::assert_preserved(&base, &after_parent);
    let notice_head = facts::persist(
        &facts::service(&db, db.owner, Role::Owner),
        db.case,
        facts::correct(&notice),
    );
    let (notice_job, notice_event) = extra::event_job(
        &mut db,
        notice_head
            .snapshot
            .metadata()
            .receipt
            .operation_id
            .as_uuid(),
    );
    let (next, second_result) = extra::revision(&mut db, &after_parent, &notice_job, notice_event);
    assert_eq!(notice_event.family, DependencyFamily::Notification);
    assert_eq!(notice_event.source_id, parent_event.source_id);
    let source = extra::observed(&next, ObservationRole::Source);
    assert_eq!(source.revision, 2);
    assert_eq!(
        source.parent_resolution.unwrap().revision,
        parent_ref.revision.get()
    );
    assert_eq!(
        extra::observed(&next, ObservationRole::NotificationParent).revision,
        2
    );
    assert_eq!(next.review_state(), DeadlineReviewState::Pending);
    worker::assert_preserved(&after_parent, &next);
    assert_eq!(
        next.definition.input.selection,
        base.definition.input.selection
    );
    dispatch::advance(&db, &parent_head);
    extra::history(
        &mut db,
        &[base, after_parent, next],
        &[first_result, second_result],
    );
}

#[test]
fn hearing_result_changes_preserve_the_selected_agreement_after_its_removal_and_withdrawal() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let first = inputs::hearing_result(&mut db, true);
    let profile = family::publish(&db, Some(db.case), TriggerField::HearingSessionEventTime);
    let source = dl::source(&db);
    extra::clear_events(&mut db);
    let mut command = dispatch::command(&db, &profile, &source, 813);
    dl::definition_mut(&mut command).input.selection = inputs::request(
        db.case,
        TriggerSourceRef::HearingResult(FactHearingRef {
            hearing_id: first.snapshot.hearing_id,
            result_id: first.snapshot.id,
            revision: first.snapshot.revision,
            agreement_id: Some(HearingResultAgreementId::from_uuid(Uuid::nil())),
        }),
    )
    .trigger;
    let base = extra::commit(&db, &command, Some(tracked::policies()), None);
    let changed = inputs::corrected_result(&db, &first);
    let (job, event) = extra::event_job(&mut db, changed.snapshot.receipt.operation_id.as_uuid());
    assert_eq!(event.family, DependencyFamily::HearingResult);
    assert_eq!(event.hearing_id, Some(first.snapshot.hearing_id.as_uuid()));
    let (next, first_result) = extra::revision(&mut db, &base, &job, event);
    worker::assert_preserved(&base, &next);
    assert_eq!(next.review_state(), DeadlineReviewState::Pending);
    assert_eq!(extra::observed(&next, ObservationRole::Source).revision, 2);
    let withdrawn = inputs::retired_result(&db, &changed);
    let (job, event) = extra::event_job(&mut db, withdrawn.snapshot.receipt.operation_id.as_uuid());
    let (last, second_result) = extra::revision(&mut db, &next, &job, event);
    worker::assert_preserved(&next, &last);
    assert!(last
        .tracking
        .as_ref()
        .unwrap()
        .review
        .reasons()
        .iter()
        .any(|r| r.dependency == TrackingDependency::Source
            && r.reason == TrackingReviewReason::DependencyRetired));
    assert_eq!(
        last.definition.input.selection,
        base.definition.input.selection
    );
    extra::history(&mut db, &[base, next, last], &[first_result, second_result]);
}
