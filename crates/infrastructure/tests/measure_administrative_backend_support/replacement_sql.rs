use super::*;

#[path = "replacement_sql_support.rs"]
mod direct;

#[test]
fn direct_replacement_requires_both_owned_rows_and_the_single_exact_new_root() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, judicial, subject, command) = setup_joint(&mut db);
    let history = MeasureDecisionRecordHistoryEvidence {
        records: MeasureRecordHistoryEvidence {
            judicial: MeasureHistoryEvidence {
                groups: vec![MeasureGroupEvidence {
                    origin: judicial.origin.clone(),
                    capture: judicial.group.clone(),
                }],
            },
            administrative: vec![],
        },
        decisions: vec![],
    };
    let capture = prepare_measure_administrative_replacement_with_decision_history(
        &RingSha256Hasher,
        &seed.actor,
        db.case,
        command,
        MeasureAdministrativeReplacementMaterial {
            context: judicial.group.review.material.context.clone(),
            subject,
        },
        &history,
    )
    .unwrap()
    .into_capture(&RingSha256Hasher, db.at)
    .unwrap();
    let before = snapshot(&mut db);
    direct::insert(&db, &capture, true, true, true)
        .expect("a complete checked replacement must pass the direct SQL guards");
    assert_eq!(snapshot(&mut db), before);
    for (marked, replacement, root) in [
        (false, true, true),
        (true, false, true),
        (true, true, false),
    ] {
        assert!(direct::insert(&db, &capture, marked, replacement, root).is_err());
        assert_eq!(snapshot(&mut db), before);
    }
}
