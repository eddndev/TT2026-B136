use super::*;
use application::{documents::StageSupportReadLimits, ApplicationError};

mod sources;
mod support;
use support::*;

#[test]
fn missing_or_altered_administrative_payload_owner_and_original_audit_fail_closed() {
    for statement in [
        "DELETE FROM case_measure_administrations",
        "DELETE FROM case_measure_operations WHERE family='a1'",
        "DELETE FROM audit_events WHERE action='measure_administrative.recorded'",
        "UPDATE case_measure_administrations SET reason='Another recorded reason'",
        "UPDATE case_measure_operations SET owner_digest=decode(repeat('a1',32),'hex') WHERE family='a1'",
        "UPDATE audit_events SET actor='altered@example.test' WHERE action='measure_administrative.recorded'",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        let (seed, _, command) = setup(&mut db);
        let original = persist(&db, seed.actor.clone(), command);
        reopened(&db, &seed.actor, &original);
        let storage = store(&db);
        let workflow = existing_service(&db, seed.actor.clone(), storage.clone());

        damage(&mut db, &ADMINISTRATIVE_TABLES, statement);

        reject_original(&mut db, &storage, &workflow, &seed.actor, &original);
    }
}

#[test]
fn losing_the_latest_corrected_row_cannot_expose_an_older_judicial_or_corrected_head() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, judicial, command) = setup(&mut db);
    let first = persist(&db, seed.actor.clone(), command);
    let latest = persist(
        &db,
        seed.actor.clone(),
        mark(corrected_reference(&first.capture), seed.command.context),
    );
    reopened(&db, &seed.actor, &latest);
    let storage = store(&db);
    let workflow = existing_service(&db, seed.actor.clone(), storage.clone());
    let judicial_store = crate::measure_fixture::store(&db);
    damage(
        &mut db,
        &ADMINISTRATIVE_TABLES,
        "DELETE FROM case_measure_revisions WHERE family='c1' AND revision=3",
    );
    let before = snapshot(&mut db);

    for previous in [
        reference(&judicial.group.measures[0]),
        corrected_reference(&first.capture),
    ] {
        let fresh = mark(previous, seed.command.context);
        assert!(storage
            .prepare(
                &seed.actor,
                db.case,
                &fresh,
                &StageSupportReadLimits::standard()
            )
            .is_err());
        assert!(workflow.prepare("session", db.case, fresh).is_err());
    }
    assert!(MeasureDecisionReadStore::get(
        judicial_store.as_ref(),
        &seed.actor,
        db.case,
        judicial.origin.decision_id
    )
    .is_err());
    assert!(MeasureDecisionStore::prepare(
        judicial_store.as_ref(),
        &seed.actor,
        db.case,
        &effect_command(
            &seed.command,
            vec![MeasureEffect::Confirm {
                previous: reference(&judicial.group.measures[0]),
            }]
        ),
        &StageSupportReadLimits::standard()
    )
    .is_err());
    assert_eq!(snapshot(&mut db), before);
    reject_original(&mut db, &storage, &workflow, &seed.actor, &first);
    reject_original(&mut db, &storage, &workflow, &seed.actor, &latest);
}

#[test]
fn computed_value_hashes_do_not_admit_malformed_correction_bytes_or_contradictory_views() {
    for statement in [
        "UPDATE case_measure_administrations
         SET correction_canonical=set_byte(correction_canonical,6,255),
             correction_digest=sha256(set_byte(correction_canonical,6,255))",
        "UPDATE case_measure_administrations
         SET correction_view=jsonb_set(correction_view,'{conditions}','\"Contradictory conditions\"'),
             correction_digest=sha256(correction_canonical)",
        "UPDATE case_measure_revisions
         SET values_view=jsonb_set(values_view,'{conditions}','\"Contradictory result\"'),
             values_digest=sha256(values_canonical) WHERE family='c1'",
    ] {
        let Some(mut db) = Fixture::new() else { return };
        let (seed, _, command) = setup(&mut db);
        let original = persist(&db, seed.actor.clone(), command);
        reopened(&db, &seed.actor, &original);
        let storage = store(&db);
        let workflow = existing_service(&db, seed.actor.clone(), storage.clone());

        damage(&mut db, &ADMINISTRATIVE_TABLES, statement);

        let hashes_match: bool = db.admin.query_one(
            "SELECT NOT EXISTS(SELECT 1 FROM case_measure_administrations
             WHERE correction_digest<>sha256(correction_canonical))
             AND NOT EXISTS(SELECT 1 FROM case_measure_revisions
             WHERE values_digest<>sha256(values_canonical))", &[],
        ).unwrap().get(0);
        assert!(hashes_match);
        reject_original(&mut db, &storage, &workflow, &seed.actor, &original);
    }
}
