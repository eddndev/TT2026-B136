mod alert_backend_support;
mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_profile_database_support;
mod hearing_database_support;
mod procedural_fact_backend_support;

use alert_backend_support::*;
use application::{
    alerts::*, deadline_currentness::DeadlineFreshness, deadline_tracking::DeadlineReviewState,
    deadlines::*, procedural_facts::*,
};
use deadline_backend_support as deadlines;
use domain::{deadline_triggers::TriggerSourceRef, identity::Role};
use procedural_fact_backend_support as facts;
use std::sync::Arc;
use time::Duration;

#[test]
fn accepted_review_then_changed_source_between_scans_starts_a_new_occurrence() {
    let Some(mut db) = fixture() else { return };
    let (source, deadline) = accepted_deadline(&db, db.owner);
    let due = deadline.calculation.result.due_at().unwrap();
    db.at = due - Duration::hours(72);
    let clock = Arc::new(MutableClock::new(db.at));
    let alerts = store(&db, clock.clone());
    let source_head = facts::persist(
        &facts::service(&db, db.owner, Role::Owner),
        db.case,
        facts::correct(&source),
    );
    drive(&alerts);
    let first = alerts.list(db.owner, query(20)).unwrap();
    assert_eq!(first.alerts.len(), 1);
    let first = first.alerts[0].clone();
    assert_eq!(first.kind, AlertKind::ReviewRequired);
    assert_eq!(first.state, AlertState::Active);
    assert_eq!(first.origin.revision, deadline.revision.get());

    db.at += Duration::seconds(1);
    clock.set(db.at);
    let mut correction = deadlines::correct(&deadline);
    deadlines::definition_mut(&mut correction)
        .input
        .selection
        .source = FactDeclaration::Known(TriggerSourceRef::Resolution(facts::resolution_ref(
        &source_head,
    )));
    let accepted = deadlines::persist(
        &deadlines::service(&db, db.owner, Role::Owner),
        db.case,
        deadlines::human(correction, Some(deadlines::FOLLOW_RESOLUTION)),
    );
    assert_eq!(accepted.review_state(), DeadlineReviewState::Accepted);
    let current = deadlines::store(&db)
        .current(db.owner, db.case, accepted.id)
        .unwrap();
    assert_eq!(
        current.operational().freshness(),
        DeadlineFreshness::Current
    );
    assert!(!current.operational().requires_review());
    assert_eq!(current.operational().due_at(), Some(due));

    db.at += Duration::seconds(1);
    clock.set(db.at);
    let ProceduralFactSnapshot::Resolution(head) = &source_head.snapshot else {
        unreachable!()
    };
    let replacement = ResolutionValues::new(ResolutionValuesInput {
        class: head.values.class().clone(),
        subtype: head.values.subtype().cloned(),
        issuer: head.values.issuer().clone(),
        issued_at: head.values.issued_at(),
        summary: facts::text("Second source correction after human review"),
        provenance: head.values.provenance().clone(),
    });
    let next_source = facts::persist(
        &facts::service(&db, db.owner, Role::Owner),
        db.case,
        ProceduralFactCommand::Resolution(ResolutionCommand::new(
            FactOperationId::new(),
            head.root.id(),
            FactChange::correct(
                head.metadata.revision,
                replacement,
                facts::text("Correct another source transcription"),
            ),
        )),
    );
    assert_eq!(next_source.snapshot.metadata().revision.get(), 3);
    let current = deadlines::store(&db)
        .current(db.owner, db.case, accepted.id)
        .unwrap();
    assert_eq!(
        current.operational().freshness(),
        DeadlineFreshness::Changed
    );
    assert!(current.operational().requires_review());
    assert!(current.operational().due_at().is_none());

    // Neither the inbox nor the scheduler observes the accepted intermediate state.
    let resolved = alerts.get(db.owner, first.id).unwrap().alert;
    assert!(matches!(
        resolved.state,
        AlertState::Resolved {
            reason: AlertResolutionReason::NoLongerEligible,
            ..
        }
    ));
    drop(alerts);
    let restarted = store(&db, clock.clone());
    drive(&restarted);
    let page = restarted.list(db.owner, query(20)).unwrap();
    assert_eq!(page.alerts.len(), 2);
    let next = page
        .alerts
        .iter()
        .find(|alert| alert.id != first.id)
        .unwrap();
    assert_eq!(next.kind, AlertKind::ReviewRequired);
    assert_eq!(next.state, AlertState::Active);
    assert_eq!(next.origin.revision, accepted.revision.get());
    assert_ne!(next.occurrence_id, first.occurrence_id);
    assert_eq!(restarted.get(db.owner, first.id).unwrap().alert, resolved);
    clock.set(db.at + Duration::seconds(2));
    drive(&restarted);
    let repeated = restarted.list(db.owner, query(20)).unwrap();
    assert_eq!(repeated.alerts.len(), 2);
    assert_eq!(
        repeated
            .alerts
            .iter()
            .filter(|alert| alert.state == AlertState::Active)
            .count(),
        1
    );
}
