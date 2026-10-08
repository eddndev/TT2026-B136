use super::*;

#[test]
fn imposition_retains_the_exact_actual_initial_hearing_anchor_on_reopen_and_replay() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let workflow = service(&db, seed.actor.clone());
    let reviewed = workflow
        .prepare("session", db.case, seed.command.clone())
        .unwrap();
    assert_eq!(reviewed.material.anchor, expected_anchor(&seed.hearing));
    let stored = workflow
        .submit("session", db.case, seed.command, confirmation(&reviewed))
        .unwrap();

    assert_eq!(stored.group.review, reviewed);
    assert_eq!(stored.group.measures.len(), 2);
    assert!(stored.measure_history.groups.is_empty());
    assert_anchor(&stored, &seed.hearing);
    reopened(&db, &seed.actor, &stored);
}
