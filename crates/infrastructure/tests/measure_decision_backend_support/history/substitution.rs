use super::*;

#[test]
fn two_to_two_substitution_preserves_whole_links_and_creates_only_successor_roots() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let first = persist(&db, seed.actor.clone(), seed.command.clone());
    let successors: Vec<_> = (0..2)
        .map(|_| MeasureProposal {
            id: MeasureId::new(),
            values: values(&seed.subject),
        })
        .collect();
    let next = command(
        &seed.command,
        vec![MeasureEffect::Substitute {
            predecessors: first.group.measures.iter().map(reference).collect(),
            successors: successors.clone(),
        }],
    );
    let expected_origin = MeasureOriginIds {
        decision_id: next.decision_id,
        operation_id: next.operation_id,
    };
    let substituted = persist(&db, seed.actor.clone(), next);

    assert_eq!(substituted.group.measures.len(), 4);
    assert_eq!(substituted.group.substitutions.len(), 1);
    let link = &substituted.group.substitutions[0];
    assert_eq!(link.predecessors.len(), 2);
    assert_eq!(link.successors.len(), 2);
    for previous in &first.group.measures {
        let current = member(&substituted, previous.result.id);
        assert_eq!(current.result.revision.get(), 2);
        assert_eq!(current.result.action, MeasureCaptureAction::SubstituteOut);
        assert_eq!(current.result.previous, Some(reference(previous)));
        assert_eq!(current.result.origin, previous.result.origin);
        assert_eq!(current.result.values, previous.result.values);
        assert_eq!(current.result.sources, previous.result.sources);
        assert_eq!(current.result.projection, previous.result.projection);
        assert_eq!(current.result.effect_key, link.effect_key);
        assert!(link.predecessors.contains(&MeasureSubstitutionPredecessor {
            previous: reference(previous),
            result: reference(current),
        }));
    }
    for proposal in &successors {
        let current = member(&substituted, proposal.id);
        assert_eq!(current.result.revision.get(), 1);
        assert_eq!(current.result.action, MeasureCaptureAction::SubstituteIn);
        assert_eq!(current.result.previous, None);
        assert_eq!(current.result.origin, expected_origin);
        assert_eq!(current.result.values, proposal.values);
        assert_eq!(current.result.sources.subject, seed.subject);
        assert_eq!(current.result.sources.supervisor, None);
        assert_eq!(current.result.effect_key, link.effect_key);
        assert!(link.successors.contains(&reference(current)));
    }
    ancestors(&substituted, &[&first]);

    let counts = db
        .admin
        .query_one(
            "SELECT (SELECT count(*) FROM case_measures WHERE case_id=$1), \
         (SELECT count(*) FROM case_measure_revisions WHERE case_id=$1), \
         (SELECT count(*) FROM audit_events WHERE action='measure_decision.recorded')",
            &[&db.case.as_uuid()],
        )
        .unwrap();
    assert_eq!(counts.get::<_, i64>(0), 4);
    assert_eq!(counts.get::<_, i64>(1), 6);
    assert_eq!(counts.get::<_, i64>(2), 2);
    reopened(&db, &seed.actor, &first);
    reopened(&db, &seed.actor, &substituted);
}
