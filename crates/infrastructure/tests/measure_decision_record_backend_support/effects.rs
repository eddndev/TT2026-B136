use super::*;

#[test]
fn later_m2_and_post_m2_correction_preserve_the_actual_last_judicial_owner() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let confirmed = persist(&db, seed.actor.clone(), seed.command.clone());
    let previous = &confirmed.group.measures[0];
    let changed = changed_values(&previous.result.values, "New judicial reporting terms");
    let modified = persist(
        &db,
        seed.actor.clone(),
        effect_command(
            &seed.command,
            vec![MeasureEffect::Modify {
                previous: reference(previous),
                values: changed.clone(),
            }],
        ),
    );
    let current = &modified.group.measures[0];
    assert_eq!(current.result.revision.get(), 4);
    assert_eq!(current.result.values, changed);
    assert_eq!(current.result.action, MeasureCaptureAction::Modify);
    assert_eq!(current.result.record_root, previous.result.record_root);
    assert_eq!(
        current.result.judicial_origin,
        previous.result.judicial_origin
    );
    assert_eq!(
        modified.record_history.decisions[0].capture,
        confirmed.group
    );

    let corrected = crate::administrative_fixture::persist(
        &db,
        seed.actor.clone(),
        correction(
            reference(current),
            seed.command.context,
            &current.result.values,
            "Corrected M2 terms",
        ),
    );
    assert_eq!(corrected.capture.review.result.revision.get(), 5);
    assert_eq!(
        corrected.capture.review.result.last_judicial.reference,
        reference(current)
    );
    assert_eq!(
        corrected
            .capture
            .review
            .result
            .last_judicial
            .owner
            .group_digest,
        modified.group.capture_digest
    );
    assert_eq!(
        corrected.capture.review.support,
        modified.group.decision.support
    );
    assert_eq!(
        corrected.capture.review.result.sources,
        current.result.sources
    );
    let last = persist(
        &db,
        seed.actor.clone(),
        effect_command(
            &seed.command,
            vec![MeasureEffect::Confirm {
                previous: corrected_reference(&corrected.capture),
            }],
        ),
    );
    assert_eq!(last.group.measures[0].result.revision.get(), 6);
    assert_eq!(
        last.group.measures[0].result.values,
        corrected.capture.review.result.values
    );
    assert_eq!(last.record_history.decisions.len(), 2);
    assert_eq!(last.record_history.records.administrative.len(), 2);
    for original in [&confirmed, &modified, &last] {
        reopened(&db, &seed.actor, original);
    }
    crate::administrative_fixture::reopened(&db, &seed.actor, &corrected);
}

#[test]
fn terminal_m2_effects_retain_c_values_and_reject_fresh_effects_from_terminal_heads() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let sibling = &seed.judicial.group.measures[1];
    let terminal = persist(
        &db,
        seed.actor.clone(),
        effect_command(
            &seed.command,
            vec![
                MeasureEffect::Revoke {
                    previous: corrected_reference(&seed.corrected.capture),
                },
                MeasureEffect::Cease {
                    previous: crate::administrative_fixture::reference(sibling),
                },
            ],
        ),
    );
    assert_eq!(terminal.group.measures.len(), 2);
    for row in &terminal.group.measures {
        let corrected = row.result.id == seed.corrected.capture.review.result.id;
        assert_eq!(
            row.result.action,
            if corrected {
                MeasureCaptureAction::Revoke
            } else {
                MeasureCaptureAction::Cease
            }
        );
        assert_eq!(
            row.result.values,
            if corrected {
                seed.corrected.capture.review.result.values.clone()
            } else {
                sibling.result.values.clone()
            }
        );
        assert!(row.result.values.validity().end().is_none());
        let command = effect_command(
            &seed.command,
            vec![MeasureEffect::Confirm {
                previous: reference(row),
            }],
        );
        let before = snapshot(&mut db);
        assert!(service(&db, seed.actor.clone())
            .prepare("session", db.case, command)
            .is_err());
        assert_eq!(snapshot(&mut db), before);
    }
    reopened(&db, &seed.actor, &terminal);
}

#[test]
fn mixed_c_and_m1_substitution_records_all_four_rows_and_only_successor_roots() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let sibling = &seed.judicial.group.measures[1];
    let successors: Vec<_> = (0..2)
        .map(|_| MeasureProposal {
            id: MeasureId::new(),
            values: seed.corrected.capture.review.result.values.clone(),
        })
        .collect();
    let command = effect_command(
        &seed.command,
        vec![MeasureEffect::Substitute {
            predecessors: vec![
                corrected_reference(&seed.corrected.capture),
                crate::administrative_fixture::reference(sibling),
            ],
            successors: successors.clone(),
        }],
    );
    let substituted = persist(&db, seed.actor.clone(), command.clone());
    assert_eq!(substituted.group.measures.len(), 4);
    assert_eq!(substituted.group.substitutions.len(), 1);
    let link = &substituted.group.substitutions[0];
    assert_eq!(link.predecessors.len(), 2);
    assert_eq!(link.successors.len(), 2);
    for proposal in &successors {
        let row = substituted
            .group
            .measures
            .iter()
            .find(|m| m.result.id == proposal.id)
            .unwrap();
        assert_eq!(row.result.revision.get(), 1);
        assert_eq!(row.result.action, MeasureCaptureAction::SubstituteIn);
        assert_eq!(
            row.result.judicial_origin.operation_id,
            command.operation_id
        );
        assert_eq!(row.result.previous, None);
        assert!(link.successors.contains(&reference(row)));
    }
    let out = substituted
        .group
        .measures
        .iter()
        .find(|m| m.result.id == seed.corrected.capture.review.result.id)
        .unwrap();
    assert_eq!(
        out.result.values,
        seed.corrected.capture.review.result.values
    );
    assert_eq!(out.result.revision.get(), 3);
    assert_eq!(out.result.action, MeasureCaptureAction::SubstituteOut);
    assert_eq!(
        db.admin
            .query_one("SELECT count(*) FROM case_measures", &[])
            .unwrap()
            .get::<_, i64>(0),
        4
    );
    reopened(&db, &seed.actor, &substituted);
}

#[test]
fn fresh_v2_imposition_and_zero_row_outcome_remain_explicit_families() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = crate::measure_fixture::setup(&mut db);
    let imposed = persist(&db, seed.actor.clone(), seed.command.clone());
    assert_eq!(imposed.group.measures.len(), 2);
    assert!(imposed.record_history.records.judicial.groups.is_empty());
    assert!(imposed.record_history.records.administrative.is_empty());
    assert!(imposed.record_history.decisions.is_empty());
    let empty = persist(
        &db,
        seed.actor.clone(),
        crate::measure_fixture::no_change(&seed.command),
    );
    assert!(empty.group.measures.is_empty());
    assert!(empty.group.substitutions.is_empty());
    assert!(empty.record_history.decisions.is_empty());
    let families: Vec<String> = db
        .admin
        .query(
            "SELECT family FROM case_measure_operations ORDER BY operation_id",
            &[],
        )
        .unwrap()
        .into_iter()
        .map(|r| r.get(0))
        .collect();
    assert_eq!(families, vec!["g2", "g2"]);
    let rows: Vec<String> = db
        .admin
        .query(
            "SELECT family FROM case_measure_revisions ORDER BY measure_id",
            &[],
        )
        .unwrap()
        .into_iter()
        .map(|r| r.get(0))
        .collect();
    assert_eq!(rows, vec!["m2", "m2"]);
    reopened(&db, &seed.actor, &imposed);
    reopened(&db, &seed.actor, &empty);
}
