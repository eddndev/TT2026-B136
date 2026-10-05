use super::*;

#[test]
fn actual_record_heads_keep_marked_revision_and_exact_original_g1_c_history() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, group, corrected, marked) = chain(&mut db);
    let service = reads(&db, seed.actor);
    let expected = administrative(&marked);
    same_detail(
        &service
            .get("session", db.case, expected.reference.id())
            .unwrap(),
        &expected,
    );
    let OwnedMeasureRecord::Administrative { capture, .. } = &expected.record else {
        unreachable!()
    };
    assert_eq!(
        capture.result.validity,
        MeasureCaptureValidity::EnteredInError
    );
    assert_eq!(capture.recorded_at.nanosecond(), 123_456_789);
    for original in [
        judicial(&group, 0),
        administrative(&corrected),
        expected.clone(),
    ] {
        same_detail(
            &service
                .exact("session", db.case, original.reference)
                .unwrap(),
            &original,
        );
    }
    let page = service
        .list("session", db.case, MeasureRecordReadQuery::default())
        .unwrap();
    let mut expected_rows = vec![expected, judicial(&group, 1)];
    expected_rows.sort_by_key(|row| row.reference.id().as_uuid());
    assert_eq!(page.items.len(), expected_rows.len());
    for (actual, expected) in page.items.iter().zip(expected_rows) {
        same_detail(actual, &expected);
    }
    assert!(!page.has_more);
    assert_eq!(page.next_after_id, None);
}
