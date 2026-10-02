use crate::{
    alert_backend_support::*, deadline_backend_support as deadlines,
    hearing_database_support as hearings, procedural_fact_backend_support as facts,
};
use application::{alerts::*, hearings::*, procedural_facts::*};
use domain::{
    deadline_triggers::TriggerSourceRef, identity::Role, procedural_time::DeclaredProceduralTime,
};
use std::sync::Arc;
use time::{format_description::well_known::Rfc3339, Duration};

fn hearing_reschedule(activate: bool) {
    let Some(mut db) = fixture() else { return };
    let before = db.at.replace_nanosecond(0).unwrap() + Duration::hours(24);
    let first = hearing(&db, before);
    let clock = Arc::new(MutableClock::new(db.at));
    let alerts = store(&db, clock.clone());
    drive(&alerts);
    assert!(!alerts.list(db.owner, query(20)).unwrap().alerts.is_empty());
    db.at += Duration::seconds(1);
    clock.set(db.at);
    let after = before + Duration::hours(12);
    let changed = hearings::persist(
        &hearings::service(&db, db.owner, Role::Owner),
        db.case,
        HearingCommand {
            operation_id: HearingOperationId::new(),
            hearing_id: first.snapshot.id,
            change: HearingChange::Replace {
                expected_revision: first.snapshot.revision,
                context: hearings::context(),
                values: hearings::values(&after.format(&Rfc3339).unwrap()),
                reason: HearingNote::new("New communicated date").unwrap(),
            },
        },
    );
    assert!(matches!(
        alerts.run_next().unwrap(),
        AlertSchedulerRun::Reconciled { .. }
    ));
    if activate {
        drive(&alerts);
        let page = alerts.list(db.owner, query(20)).unwrap();
        assert!(page.alerts.iter().any(|record| {
            record.state == AlertState::Active
                && record.origin.revision == changed.snapshot.revision.get()
                && matches!(record.kind, AlertKind::Upcoming { activity_at, .. } if activity_at == after)
        }));
        assert!(page
            .alerts
            .iter()
            .all(|record| matches!(record.kind, AlertKind::Upcoming { .. })));
    }
    let before_reopen = atomicity::snapshot(&mut db);
    drop(alerts);
    let reopened = store(&db, clock);
    assert_eq!(atomicity::snapshot(&mut db), before_reopen);
    drop(reopened);
}

#[test]
fn hearing_reschedule_plans_survive_inventory_reopen_without_rewriting_rows() {
    hearing_reschedule(false);
}

#[test]
fn hearing_reschedule_activated_alerts_survive_inventory_reopen() {
    hearing_reschedule(true);
}

#[test]
fn deadline_due_change_remains_deliverable_and_survives_inventory_reopen() {
    let Some(mut db) = fixture() else { return };
    let (source, deadline) = accepted_deadline(&db, db.owner);
    let before = deadline.calculation.result.due_at().unwrap();
    db.at = before - Duration::hours(12);
    let clock = Arc::new(MutableClock::new(db.at));
    let alerts = store(&db, clock.clone());
    drive(&alerts);
    let ProceduralFactSnapshot::Resolution(head) = &source.snapshot else {
        unreachable!()
    };
    db.at += Duration::seconds(1);
    clock.set(db.at);
    let changed_source = facts::persist(
        &facts::service(&db, db.owner, Role::Owner),
        db.case,
        ProceduralFactCommand::Resolution(ResolutionCommand::new(
            FactOperationId::new(),
            head.root.id(),
            FactChange::correct(
                head.metadata.revision,
                ResolutionValues::new(ResolutionValuesInput {
                    class: head.values.class().clone(),
                    subtype: head.values.subtype().cloned(),
                    issuer: head.values.issuer().clone(),
                    issued_at: DeclaredProceduralTime::date("2026-01-07".parse().unwrap(), None)
                        .unwrap(),
                    summary: facts::text("Correct source date"),
                    provenance: head.values.provenance().clone(),
                }),
                facts::text("Correct transcribed date"),
            ),
        )),
    );
    let mut correction = deadlines::correct(&deadline);
    deadlines::definition_mut(&mut correction)
        .input
        .selection
        .source = FactDeclaration::Known(TriggerSourceRef::Resolution(facts::resolution_ref(
        &changed_source,
    )));
    let changed = deadlines::persist(
        &deadlines::service(&db, db.owner, Role::Owner),
        db.case,
        deadlines::human(correction, Some(deadlines::FOLLOW_RESOLUTION)),
    );
    let after = changed.calculation.result.due_at().unwrap();
    assert_ne!(before, after);
    assert!(after > db.at && after - db.at <= Duration::hours(48));
    drive(&alerts);
    let page = alerts.list(db.owner, query(20)).unwrap();
    let change_notice = page
        .alerts
        .iter()
        .find(|record| {
            record.state == AlertState::Active
                && matches!(record.kind, AlertKind::DueChangedSoon {
                previous_due_at, current_due_at,
            } if previous_due_at == before && current_due_at == after)
        })
        .expect("deadline movement must retain its change notice")
        .clone();
    assert_eq!(change_notice.origin.revision, changed.revision.get());
    let before_reopen = atomicity::snapshot(&mut db);
    drop(alerts);
    let reopened = store(&db, clock);
    assert_eq!(atomicity::snapshot(&mut db), before_reopen);
    assert_eq!(
        reopened.get(db.owner, change_notice.id).unwrap().alert,
        change_notice
    );
}
