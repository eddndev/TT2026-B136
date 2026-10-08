use super::*;

#[test]
fn original_v1_and_v2_replay_after_mixed_descendants_preserves_each_wire_family() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let first = persist(&db, seed.actor.clone(), seed.command.clone());
    let next = persist(
        &db,
        seed.actor.clone(),
        effect_command(
            &seed.command,
            vec![MeasureEffect::Confirm {
                previous: reference(&first.group.measures[0]),
            }],
        ),
    );
    let replay = service_with_format(
        &db,
        seed.actor.clone(),
        FormatCheck(Some(Box::new(|| {
            panic!("original V1 replay must skip support admission")
        }))),
    );
    let command = seed.judicial.group.review.command.clone();
    assert_eq!(
        replay.prepare("session", db.case, command.clone()).unwrap(),
        MeasureDecisionRecordReview::V1(Box::new(seed.judicial.group.review.clone()))
    );
    let result = replay
        .submit(
            "session",
            db.case,
            command,
            crate::measure_fixture::confirmation(&seed.judicial.group.review),
        )
        .unwrap();
    assert_eq!(
        result,
        MeasureDecisionRecordReceipt::V1(Box::new(seed.judicial.clone()))
    );
    for original in [&first, &next] {
        reopened(&db, &seed.actor, original);
    }
    crate::administrative_fixture::reopened(&db, &seed.actor, &seed.corrected);
    let legacy = crate::measure_fixture::reads(&db, seed.actor)
        .get("session", db.case, seed.judicial.origin.decision_id)
        .unwrap();
    assert_eq!(legacy, seed.judicial);
}

#[test]
fn fresh_v2_requires_current_valid_exact_heads_but_original_replay_survives_later_mark() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let first = persist(&db, seed.actor.clone(), seed.command.clone());
    let stale = effect_command(
        &seed.command,
        vec![MeasureEffect::Confirm {
            previous: corrected_reference(&seed.corrected.capture),
        }],
    );
    let before = snapshot(&mut db);
    assert!(service(&db, seed.actor.clone())
        .prepare("session", db.case, stale)
        .is_err());
    assert_eq!(snapshot(&mut db), before);
    let marked = crate::administrative_fixture::persist(
        &db,
        seed.actor.clone(),
        mark(reference(&first.group.measures[0]), seed.command.context),
    );
    let invalid = effect_command(
        &seed.command,
        vec![MeasureEffect::Confirm {
            previous: corrected_reference(&marked.capture),
        }],
    );
    let before = snapshot(&mut db);
    assert!(service(&db, seed.actor.clone())
        .prepare("session", db.case, invalid)
        .is_err());
    assert_eq!(snapshot(&mut db), before);
    reopened(&db, &seed.actor, &first);
    crate::administrative_fixture::reopened(&db, &seed.actor, &marked);
}

#[test]
fn admission_race_rechecks_c_head_and_rejected_submit_leaves_no_partial_g2_owner() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let MeasureDecisionRecordReview::V2(review) = service(&db, seed.actor.clone())
        .prepare("session", db.case, seed.command.clone())
        .unwrap()
    else {
        panic!("expected V2 preparation")
    };
    let advanced = crate::administrative_fixture::persist(
        &db,
        seed.actor.clone(),
        correction(
            corrected_reference(&seed.corrected.capture),
            seed.command.context,
            &seed.corrected.capture.review.result.values,
            "Later correction wins the head",
        ),
    );
    let before = snapshot(&mut db);
    assert!(service(&db, seed.actor.clone())
        .submit("session", db.case, seed.command, confirmation(&review),)
        .is_err());
    assert_eq!(snapshot(&mut db), before);
    crate::administrative_fixture::reopened(&db, &seed.actor, &advanced);
}
