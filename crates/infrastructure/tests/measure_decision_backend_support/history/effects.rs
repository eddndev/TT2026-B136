use super::*;
use domain::hearings::HearingParticipantRef;

#[test]
fn modification_retains_identity_and_resolves_declared_terms_and_real_supervisor() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let first = persist(&db, seed.actor.clone(), seed.command.clone());
    let previous = &first.group.measures[0];
    let old = &previous.result.values;
    let changed = MeasureValues::new(MeasureValuesInput {
        subject: old.subject(),
        kind: old.kind(),
        conditions: note("Revised reporting terms"),
        validity: MeasureValidity::new(
            declared_time(),
            note("Revised validity declaration"),
            Some(declared_time()),
        )
        .unwrap(),
        supervision: MeasureSupervision::Known {
            participant: HearingParticipantRef::new(
                seed.supervisor.id(),
                seed.supervisor.revision_number(),
            ),
            statement: note("Declared supervisor from exact directory capture"),
        },
    });
    let mut next = command(
        &seed.command,
        vec![MeasureEffect::Modify {
            previous: reference(previous),
            values: changed.clone(),
        }],
    );
    let support = upload(&db, db.case, "measure-modification.pdf");
    next.values = MeasureDecisionValues::new(MeasureDecisionValuesInput {
        authority: note("Declared court"),
        declared_at: declared_time(),
        justification: note("Declared modification reasons"),
        support: HearingSupportRef::new(
            DocumentVersionRef {
                id: support.id,
                version: support.version,
            },
            support.digest,
        ),
        locator: note("Page 2"),
    });
    let modified = persist(&db, seed.actor.clone(), next);
    let result = &modified.group.measures[0].result;

    assert_eq!(result.id, previous.result.id);
    assert_eq!(result.revision.get(), 2);
    assert_eq!(result.action, MeasureCaptureAction::Modify);
    assert_eq!(result.origin, previous.result.origin);
    assert_eq!(result.previous, Some(reference(previous)));
    assert_eq!(result.values, changed);
    assert_eq!(result.sources.subject, seed.subject);
    assert_eq!(result.sources.supervisor.as_ref(), Some(&seed.supervisor));
    assert_eq!(modified.group.decision.support.reference.id, support.id);
    assert_eq!(modified.group.decision.support.digest, support.digest);
    ancestors(&modified, &[&first]);
    reopened(&db, &seed.actor, &first);
    reopened(&db, &seed.actor, &modified);
}

fn terminal(action: MeasureCaptureAction) {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let first = persist(&db, seed.actor.clone(), seed.command.clone());
    let previous = &first.group.measures[0];
    let effect = match action {
        MeasureCaptureAction::Revoke => MeasureEffect::Revoke {
            previous: reference(previous),
        },
        MeasureCaptureAction::Cease => MeasureEffect::Cease {
            previous: reference(previous),
        },
        _ => panic!("expected terminal declaration"),
    };
    let final_group = persist(
        &db,
        seed.actor.clone(),
        command(&seed.command, vec![effect]),
    );
    let current = &final_group.group.measures[0];
    assert_eq!(current.result.action, action);
    assert_eq!(current.result.revision.get(), 2);
    assert_eq!(current.result.origin, previous.result.origin);
    assert_eq!(current.result.previous, Some(reference(previous)));
    assert_eq!(current.result.values, previous.result.values);
    assert_eq!(current.result.sources, previous.result.sources);
    assert!(current.result.values.validity().end().is_none());
    ancestors(&final_group, &[&first]);
    reopened(&db, &seed.actor, &final_group);

    let forbidden = command(
        &seed.command,
        vec![MeasureEffect::Confirm {
            previous: reference(current),
        }],
    );
    let workflow = service(&db, seed.actor);
    let before = snapshot(&mut db);
    assert!(workflow.prepare("session", db.case, forbidden).is_err());
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn revocation_retains_declared_values_without_inventing_an_end_and_is_terminal() {
    terminal(MeasureCaptureAction::Revoke);
}

#[test]
fn cessation_retains_declared_values_without_inventing_an_end_and_is_terminal() {
    terminal(MeasureCaptureAction::Cease);
}
