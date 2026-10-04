use crate::alert_resource_hearing_support::*;
use application::{
    alerts::*,
    cases::{CaseRepository, CaseRevisionExpectation},
    ApplicationError,
};
use domain::{case_administration::CaseAdministrativeStatus, identity::Role};
use std::sync::Arc;
use time::Duration;

#[test]
fn own_upcoming_uses_hearing_preferences_and_keeps_one_exact_occurrence_after_restart() {
    let Some(mut db) = Fixture::new() else { return };
    let due = db.at.replace_nanosecond(0).unwrap() + Duration::days(5);
    let (_, created) = create(&mut db, due, None);
    let clock = Arc::new(MutableClock::new(due - Duration::hours(12)));
    let alerts = store(&db, clock.clone());
    let mut prefs = AlertPreferenceValues::default();
    prefs.deadline_upcoming.channels = AlertChannels {
        internal: false,
        email: false,
    };
    prefs.hearing_upcoming.channels = AlertChannels {
        internal: true,
        email: false,
    };
    alerts
        .save_preferences(
            db.owner,
            AlertPreferenceCommand {
                operation_id: operation(),
                expected_revision: 0,
                values: prefs,
            },
        )
        .unwrap();
    drive(&alerts);
    let rows = own_alerts(&alerts, db.owner);
    assert_eq!(rows.len(), 1);
    let first = rows[0].clone();
    assert_eq!(first.subject, subject(&created));
    assert_eq!(first.origin.revision, 1);
    assert_eq!(first.origin.evidence_digest, created.origin.capture_digest);
    assert!(
        matches!(first.kind, AlertKind::Upcoming { activity_at, lead_hours }
        if activity_at == due && lead_hours.get() == 24)
    );
    assert_eq!(first.email, AlertEmailStatus::Disabled);
    let command = AlertReadCommand {
        operation_id: operation(),
        alert_id: first.id,
    };
    let read = alerts.mark_read(db.owner, command).unwrap();
    drop(alerts);
    clock.set(due - Duration::hours(11));
    let reopened = store(&db, clock);
    drive(&reopened);
    let rows = own_alerts(&reopened, db.owner);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, first.id);
    assert_eq!(rows[0].occurrence_id, first.occurrence_id);
    assert_eq!(rows[0].read_at, read.alert.read_at);
    assert_eq!(
        reopened.mark_read(db.owner, command).unwrap().alert.read_at,
        read.alert.read_at
    );
}

#[test]
fn disabled_hearing_preferences_do_not_fall_back_to_deadline_preferences() {
    let Some(mut db) = Fixture::new() else { return };
    let due = db.at.replace_nanosecond(0).unwrap() + Duration::days(5);
    create(&mut db, due, None);
    let alerts = store(&db, Arc::new(MutableClock::new(due - Duration::hours(12))));
    let mut prefs = AlertPreferenceValues::default();
    prefs.hearing_upcoming.channels = AlertChannels {
        internal: false,
        email: false,
    };
    alerts
        .save_preferences(
            db.owner,
            AlertPreferenceCommand {
                operation_id: operation(),
                expected_revision: 0,
                values: prefs,
            },
        )
        .unwrap();
    drive(&alerts);
    assert!(own_alerts(&alerts, db.owner).is_empty());
    let own_outbox: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM alert_email_outbox o
        JOIN alert_notifications n ON n.id=o.alert_id WHERE n.kind=2",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(own_outbox, 0);
}

#[test]
fn unlink_archive_and_case_close_preserve_upcoming_and_its_immutable_origin() {
    let Some(mut db) = Fixture::new() else { return };
    let due = db.at.replace_nanosecond(0).unwrap() + Duration::days(5);
    let (captures, created) = create(&mut db, due, None);
    crate::resource_activity_support::persist(
        &crate::resource_activity_support::service(&db, db.owner, Role::Owner),
        db.case,
        captures.resource.id,
        crate::resource_activity_support::unlink(&created.association, captures.head.revision),
    );
    captures.archive(&db);
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(1),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    let clock = Arc::new(MutableClock::new(due - Duration::hours(12)));
    let alerts = store(&db, clock.clone());
    drive(&alerts);
    let rows = own_alerts(&alerts, db.owner);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].subject, subject(&created));
    assert_eq!(rows[0].state, AlertState::Active);
    assert_eq!(
        rows[0].origin.evidence_digest,
        created.origin.capture_digest
    );
    let id = rows[0].id;
    drop(alerts);
    clock.set(due);
    let restored = store(&db, clock);
    let expired = restored.get(db.owner, id).unwrap().alert;
    assert!(matches!(
        expired.state,
        AlertState::Resolved {
            reason: AlertResolutionReason::NoLongerEligible,
            ..
        }
    ));
    assert_eq!(expired.origin, rows[0].origin);
    assert_eq!(expired.subject, rows[0].subject);
}

#[test]
fn active_staff_members_receive_own_alerts_and_revocation_denies_detail_and_read() {
    let Some(mut db) = Fixture::new() else { return };
    let due = db.at.replace_nanosecond(0).unwrap() + Duration::days(5);
    create(&mut db, due, None);
    let litigator = db.user("litigator", true);
    let para = db.user("paralegal", true);
    let global_owner = db.user("owner", false);
    let outside = db.user("litigator", false);
    let client = db.user("client", true);
    let inactive = db.user("paralegal", true);
    db.admin.execute("UPDATE users SET active=false,revision=revision+1,auth_generation=auth_generation+1 WHERE id=$1", &[&inactive.as_uuid()]).unwrap();
    let alerts = store(&db, Arc::new(MutableClock::new(due - Duration::hours(12))));
    drive(&alerts);
    for user in [db.owner, litigator, para] {
        assert_eq!(own_alerts(&alerts, user).len(), 1);
    }
    for user in [global_owner, outside] {
        assert!(own_alerts(&alerts, user).is_empty());
    }
    assert!(matches!(
        alerts.list(client, query(20)),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(alerts.list(inactive, query(20)).is_err());
    let id = own_alerts(&alerts, litigator)[0].id;
    db.store()
        .remove_member(db.case, litigator, db.owner, db.at)
        .unwrap();
    assert!(own_alerts(&alerts, litigator).is_empty());
    assert!(matches!(
        alerts.get(litigator, id),
        Err(ApplicationError::Alert(AlertError::NotFound))
    ));
    assert!(matches!(
        alerts.mark_read(
            litigator,
            AlertReadCommand {
                operation_id: operation(),
                alert_id: id,
            }
        ),
        Err(ApplicationError::Alert(AlertError::NotFound))
    ));
}

#[test]
fn all_three_families_can_share_uuid_without_occurrence_or_cursor_collision() {
    use crate::{deadline_backend_support as deadlines, hearing_database_support as hearings};
    use application::{
        deadlines::DeadlineId,
        hearings::{HearingChange, HearingId},
    };
    let Some(mut db) = Fixture::new() else { return };
    let (captures, mut command) = own::setup(&mut db);
    let profile = deadlines::profile(&db);
    let source = deadlines::source(&db);
    let mut deadline = deadlines::command(&db, &profile, &source);
    deadline.deadline_id = DeadlineId::new();
    let deadline = deadlines::persist(
        &deadlines::service(&db, db.owner, Role::Owner),
        db.case,
        deadlines::human(deadline, Some(deadlines::FOLLOW_RESOLUTION)),
    );
    let due = deadline.calculation.result.due_at().unwrap();
    let id = deadline.id.as_uuid();
    let mut hearing = hearings::schedule();
    hearing.hearing_id = HearingId::from_uuid(id);
    if let HearingChange::Schedule { values, .. } = &mut hearing.change {
        *values = hearings::values(
            &due.format(&time::format_description::well_known::Rfc3339)
                .unwrap(),
        );
    }
    hearings::persist(
        &hearings::service(&db, db.owner, Role::Owner),
        db.case,
        hearing,
    );
    command.hearing_id = domain::resource_hearings::ResourceHearingId::from_uuid(id);
    let v = &command.values;
    command.values = domain::resource_hearings::ResourceHearingValues::new(
        domain::resource_hearings::ResourceHearingValuesInput {
            kind: v.kind(),
            scheduled_at: domain::hearings::HearingTime::new(due).unwrap(),
            modality: v.modality(),
            venue: v.venue().clone(),
            note: v.note().cloned(),
            participants: vec![],
            scheduling_basis: v.scheduling_basis().clone(),
        },
    )
    .unwrap();
    let created = own::submit(&db, command);
    assert_eq!(created.origin.resource_id, captures.resource.id);
    let alerts = store(&db, Arc::new(MutableClock::new(due - Duration::hours(12))));
    drive(&alerts);
    let rows = db
        .admin
        .query(
            "SELECT kind,occurrence_key,occurrence_id FROM alert_schedule
        WHERE subject_id=$1 AND status='activated' ORDER BY kind",
            &[&id],
        )
        .unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(
        rows.iter().map(|r| r.get::<_, i16>(0)).collect::<Vec<_>>(),
        [0, 1, 2]
    );
    assert_eq!(
        rows.iter()
            .map(|r| r.get::<_, String>(1))
            .collect::<std::collections::HashSet<_>>()
            .len(),
        3
    );
    assert_eq!(
        rows.iter()
            .map(|r| r.get::<_, uuid::Uuid>(2))
            .collect::<std::collections::HashSet<_>>()
            .len(),
        3
    );
    drop(alerts);
    open(&db).unwrap();
}

#[test]
fn restart_during_own_recipient_page_finishes_each_member_once() {
    let Some(mut db) = Fixture::new() else { return };
    let due = db.at.replace_nanosecond(0).unwrap() + Duration::days(5);
    let (_, created) = create(&mut db, due, None);
    let mut users = vec![db.owner];
    for _ in 0..32 {
        users.push(db.user("paralegal", true));
    }
    let clock = Arc::new(MutableClock::new(due - Duration::hours(12)));
    let alerts = store(&db, clock.clone());
    let mut resumed_cursor = false;
    for _ in 0..10 {
        alerts.run_next().unwrap();
        resumed_cursor = db
            .admin
            .query_one(
                "SELECT active_kind=2 AND active_id=$1
            AND after_recipient IS NOT NULL FROM alert_scan_cursor",
                &[&created.origin.hearing_id.as_uuid()],
            )
            .unwrap()
            .get::<_, Option<bool>>(0)
            .unwrap_or(false);
        if resumed_cursor {
            break;
        }
    }
    assert!(
        resumed_cursor,
        "scanner did not retain a partial own recipient page"
    );
    drop(alerts);
    let restarted = store(&db, clock);
    let mut idle = false;
    for _ in 0..100 {
        if matches!(restarted.run_next().unwrap(), AlertSchedulerRun::Idle) {
            idle = true;
            break;
        }
    }
    assert!(idle, "bounded own scan did not finish");
    for user in users {
        assert_eq!(own_alerts(&restarted, user).len(), 1);
    }
    let count: i64 = db
        .admin
        .query_one("SELECT count(*) FROM alert_notifications WHERE kind=2", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 33);
}
