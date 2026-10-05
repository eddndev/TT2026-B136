use super::*;
use application::{
    measure_corrections::{MeasureAdministrativeError, MeasureAdministrativeWorkflow},
    ApplicationError,
};
use domain::{crypto::DocumentVersionRef, hearings::HearingSupportRef};

#[test]
fn correction_and_mark_after_modify_retain_the_latest_actual_m2_support() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let confirmed = persist(&db, seed.actor.clone(), seed.command.clone());
    let previous = &confirmed.group.measures[0];
    let mut command = effect_command(
        &seed.command,
        vec![MeasureEffect::Modify {
            previous: reference(previous),
            values: changed_values(
                &previous.result.values,
                "Judicial terms declared by the latest support",
            ),
        }],
    );
    let document = crate::measure_fixture::upload(&db, db.case, "later-m2-support.pdf");
    let old = &command.values;
    command.values = MeasureDecisionValues::new(MeasureDecisionValuesInput {
        authority: old.authority().clone(),
        declared_at: old.declared_at().clone(),
        justification: old.justification().clone(),
        support: HearingSupportRef::new(
            DocumentVersionRef {
                id: document.id,
                version: document.version,
            },
            document.digest,
        ),
        locator: note("Page 2"),
    });
    let latest = persist(&db, seed.actor.clone(), command);
    assert_ne!(
        latest.group.decision.support.reference,
        seed.judicial.group.decision.support.reference
    );
    let selected = &latest.group.measures[0];
    let corrected = crate::administrative_fixture::persist(
        &db,
        seed.actor.clone(),
        correction(
            reference(selected),
            seed.command.context,
            &selected.result.values,
            "Corrected latest M2 wording",
        ),
    );
    let marked = crate::administrative_fixture::persist(
        &db,
        seed.actor.clone(),
        mark(
            corrected_reference(&corrected.capture),
            seed.command.context,
        ),
    );
    for operation in [&corrected, &marked] {
        let review = &operation.capture.review;
        assert_eq!(review.support, latest.group.decision.support);
        assert_eq!(review.result.last_judicial.reference, reference(selected));
        assert_eq!(
            review.result.last_judicial.owner.group_digest,
            latest.group.capture_digest
        );
        assert_eq!(review.result.sources, selected.result.sources);
        assert_eq!(review.result.record_root, selected.result.record_root);
        assert_eq!(
            review.result.judicial_origin,
            selected.result.judicial_origin
        );
        assert!(operation
            .record_history
            .decisions
            .iter()
            .any(|g| g.capture == latest.group));
        crate::administrative_fixture::reopened(&db, &seed.actor, operation);
    }
}

#[test]
fn real_g2_anchor_dependant_blocks_correction_of_the_still_current_exact_m2() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let first = persist(&db, seed.actor.clone(), seed.command.clone());
    let selected = &first.group.measures[0];
    let hearing = super::anchors::hearing(&db, &seed, reference(selected));
    let mut command = crate::measure_fixture::no_change(&seed.command);
    super::anchors::attach(&mut command, &hearing);
    let dependant = persist(&db, seed.actor.clone(), command);
    assert!(dependant.group.measures.is_empty());
    let head: i64 = db
        .admin
        .query_one(
            "SELECT max(revision) FROM case_measure_revisions WHERE measure_id=$1",
            &[&selected.result.id.as_uuid()],
        )
        .unwrap()
        .get(0);
    assert_eq!(head, i64::from(selected.result.revision.get()));
    let candidate = correction(
        reference(selected),
        seed.command.context,
        &selected.result.values,
        "Blocked after the exact capture was used",
    );
    let workflow = crate::administrative_fixture::service(&db, seed.actor.clone());
    let before = snapshot(&mut db);
    assert!(matches!(
        workflow.prepare("session", db.case, candidate),
        Err(ApplicationError::MeasureAdministrative(
            MeasureAdministrativeError::KnownDependants
        ))
    ));
    assert_eq!(snapshot(&mut db), before);
    reopened(&db, &seed.actor, &dependant);
}
