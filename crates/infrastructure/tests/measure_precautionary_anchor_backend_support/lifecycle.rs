use super::*;

#[test]
fn actual_imposition_hearing_anchors_a_durable_measure_group() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let workflow = service(&db, seed.seed.actor.clone());
    let review = workflow
        .prepare("session", db.case, seed.command.clone())
        .unwrap();
    assert_eq!(review.material.anchor, expected_anchor(&seed.hearing));
    let stored = workflow
        .submit("session", db.case, seed.command, confirmation(&review))
        .unwrap();

    assert_eq!(stored.group.review, review);
    assert_eq!(stored.group.measures.len(), 2);
    assert!(stored.measure_history.groups.is_empty());
    assert_anchor(&stored, &seed.hearing);
    reopened(&db, &seed.seed.actor, &stored);
}
