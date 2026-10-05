use super::*;
use domain::precautionary_measures::{
    MeasureDecisionOutcome, MeasureDecisionOutcomeInput, MeasureEffect, MeasureProposal,
};
use uuid::Uuid;

fn id(value: u128) -> MeasureId {
    MeasureId::from_uuid(Uuid::from_u128(value))
}

#[test]
fn record_pages_order_stable_measure_ids_and_exclude_no_change_groups() {
    let Some(mut db) = Fixture::new() else { return };
    let mut seed = crate::measure_fixture::setup(&mut db);
    seed.command.outcome = MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(
        [30, 10, 20]
            .into_iter()
            .map(|value| {
                MeasureEffect::Impose(MeasureProposal {
                    id: id(value),
                    values: crate::measure_fixture::values(&seed.subject),
                })
            })
            .collect(),
    ))
    .unwrap();
    let group = crate::measure_fixture::persist(&db, seed.actor.clone(), seed.command.clone());
    let changed = crate::administrative_fixture::persist(
        &db,
        seed.actor.clone(),
        crate::administrative_fixture::mark(
            crate::administrative_fixture::reference(&group.group.measures[1]),
            seed.command.context,
        ),
    );
    let no_change = crate::measure_fixture::persist(
        &db,
        seed.actor.clone(),
        crate::measure_fixture::no_change(&seed.command),
    );
    assert!(no_change.group.measures.is_empty());
    let mut expected: Vec<_> = (0..3).map(|index| judicial(&group, index)).collect();
    expected[1] = administrative(&changed);
    expected.sort_by_key(|row| row.reference.id().as_uuid());
    let service = reads(&db, seed.actor);
    let first = service
        .list(
            "session",
            db.case,
            MeasureRecordReadQuery::new(2, None).unwrap(),
        )
        .unwrap();
    assert_eq!(first.case_id, db.case);
    assert_eq!(first.items.len(), 2);
    for (actual, expected) in first.items.iter().zip(&expected[..2]) {
        same_detail(actual, expected);
    }
    assert!(first.has_more);
    assert_eq!(first.next_after_id, Some(id(20)));
    let last = service
        .list(
            "session",
            db.case,
            MeasureRecordReadQuery::new(2, first.next_after_id).unwrap(),
        )
        .unwrap();
    assert_eq!(last.items.len(), 1);
    same_detail(&last.items[0], &expected[2]);
    assert!(!last.has_more);
    assert_eq!(last.next_after_id, None);
    let between = service
        .list(
            "session",
            db.case,
            MeasureRecordReadQuery::new(1, Some(id(15))).unwrap(),
        )
        .unwrap();
    same_detail(&between.items[0], &expected[1]);
    assert!(between.has_more);
    assert_eq!(between.next_after_id, Some(id(20)));
    let empty = service
        .list(
            "session",
            db.case,
            MeasureRecordReadQuery::new(2, Some(id(30))).unwrap(),
        )
        .unwrap();
    assert!(empty.items.is_empty());
    assert!(!empty.has_more);
    assert_eq!(empty.next_after_id, None);
}
