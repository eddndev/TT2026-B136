use super::*;

pub(super) fn replace(db: &Fixture, previous: &HearingDetail) -> HearingDetail {
    let old = &previous.snapshot.values;
    let values = HearingValues::new(HearingValuesInput {
        kind: old.kind(),
        scheduled_at: old.scheduled_at(),
        modality: old.modality(),
        venue: domain::hearings::HearingVenue::new("Replacement Initial courtroom").unwrap(),
        note: old.note().cloned(),
        participants: old.participants().to_vec(),
        conviction_basis: old.conviction_basis().cloned(),
    })
    .unwrap();
    crate::hearing_database_support::persist(
        &hearing_service(db),
        db.case,
        HearingCommand {
            operation_id: HearingOperationId::new(),
            hearing_id: previous.snapshot.id,
            change: HearingChange::Replace {
                expected_revision: previous.snapshot.revision,
                context: crate::hearing_database_support::context(),
                values,
                reason: note("Updated declared courtroom"),
            },
        },
    )
}

pub(super) fn cancel(db: &Fixture, previous: &HearingDetail) -> HearingDetail {
    crate::hearing_database_support::persist(
        &hearing_service(db),
        db.case,
        HearingCommand {
            operation_id: HearingOperationId::new(),
            hearing_id: previous.snapshot.id,
            change: HearingChange::Cancel {
                expected_revision: previous.snapshot.revision,
                reason: note("Cancelled declared appointment"),
            },
        },
    )
}

#[test]
fn replaced_initial_anchor_remains_exact_after_later_cancellation() {
    let Some(mut db) = Fixture::new() else { return };
    let mut seed = setup(&mut db);
    let replacement = replace(&db, &seed.hearing);
    seed.command.anchor = Some(anchor(&replacement));
    let stored = persist(&db, seed.actor.clone(), seed.command);
    let original_bytes = measure_decision_group_bytes(&stored.group).unwrap();
    let cancelled = cancel(&db, &replacement);

    assert_eq!(cancelled.snapshot.revision.get(), 3);
    assert_eq!(cancelled.snapshot.status, HearingStatus::Cancelled);
    assert_anchor(&stored, &replacement);
    reopened(&db, &seed.actor, &stored);
    let actual = reads(&db, seed.actor)
        .get("session", db.case, stored.origin.decision_id)
        .unwrap();
    assert_eq!(
        measure_decision_group_bytes(&actual.group).unwrap(),
        original_bytes
    );
}

#[test]
fn cancelled_initial_anchor_supports_a_real_zero_member_decision() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let replacement = replace(&db, &seed.hearing);
    let cancelled = cancel(&db, &replacement);
    let mut command = no_change(&seed.command);
    command.anchor = Some(anchor(&cancelled));

    let stored = persist(&db, seed.actor.clone(), command);

    assert_anchor(&stored, &cancelled);
    assert!(stored.group.measures.is_empty());
    assert!(stored.group.substitutions.is_empty());
    assert!(stored.measure_history.groups.is_empty());
    let counts = db
        .admin
        .query_one(
            "SELECT (SELECT count(*) FROM case_measure_operations),
         (SELECT count(*) FROM case_measure_decisions),
         (SELECT count(*) FROM case_measures),
         (SELECT count(*) FROM case_measure_revisions),
         (SELECT count(*) FROM audit_events WHERE action='measure_decision.recorded')",
            &[],
        )
        .unwrap();
    assert_eq!(counts.get::<_, i64>(0), 1);
    assert_eq!(counts.get::<_, i64>(1), 1);
    assert_eq!(counts.get::<_, i64>(2), 0);
    assert_eq!(counts.get::<_, i64>(3), 0);
    assert_eq!(counts.get::<_, i64>(4), 1);
    reopened(&db, &seed.actor, &stored);
}

#[test]
fn fresh_decision_can_select_an_old_initial_revision_after_a_cancelled_head() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let replacement = replace(&db, &seed.hearing);
    let cancelled = cancel(&db, &replacement);
    assert_eq!(cancelled.snapshot.revision.get(), 3);

    let stored = persist(&db, seed.actor.clone(), seed.command);

    assert_anchor(&stored, &seed.hearing);
    assert_eq!(seed.hearing.snapshot.revision.get(), 1);
    assert!(stored.measure_history.groups.is_empty());
    reopened(&db, &seed.actor, &stored);
}

#[test]
fn anchored_confirmation_retains_real_measure_ancestry_and_its_selected_initial_revision() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let first = persist(&db, seed.actor.clone(), seed.command.clone());
    let replacement = replace(&db, &seed.hearing);
    let mut next = fresh(&seed.command);
    next.anchor = Some(anchor(&replacement));
    next.outcome = MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(
        first
            .group
            .measures
            .iter()
            .map(|capture| MeasureEffect::Confirm {
                previous: domain::precautionary_hearings::PrecautionaryMeasureRef::new(
                    capture.result.id,
                    capture.result.revision,
                    capture.capture_digest,
                ),
            })
            .collect(),
    ))
    .unwrap();

    let confirmed = persist(&db, seed.actor.clone(), next);

    assert_anchor(&confirmed, &replacement);
    assert_eq!(confirmed.measure_history.groups.len(), 1);
    assert_eq!(confirmed.measure_history.groups[0].origin, first.origin);
    assert_eq!(confirmed.measure_history.groups[0].capture, first.group);
    for (before, after) in first.group.measures.iter().zip(&confirmed.group.measures) {
        assert_eq!(after.result.id, before.result.id);
        assert_eq!(after.result.revision.get(), 2);
        assert_eq!(after.result.action, MeasureCaptureAction::Confirm);
        assert_eq!(after.result.origin, before.result.origin);
        assert_eq!(after.result.values, before.result.values);
        assert_eq!(after.result.sources, before.result.sources);
    }
    reopened(&db, &seed.actor, &confirmed);
}
