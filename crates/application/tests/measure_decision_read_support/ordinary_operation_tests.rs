use super::*;
use application::hearings::{HearingDetail, HearingId, HearingOperationId};

fn anchored_initial(serial: u128, detail: HearingDetail) -> MeasureDecisionStoredOperation {
    let mut fixture = root_fixture(serial);
    crate::decision_anchor_support::attach_initial(&mut fixture, detail);
    stored(fixture.capture(), empty_history())
}

fn list_anchors(
    first: HearingDetail,
    second: HearingDetail,
) -> Result<MeasureDecisionPage, ApplicationError> {
    let items = vec![anchored_initial(10, first), anchored_initial(20, second)];
    let case_id = items[0].group.review.case_id;
    let returned = page(case_id, items);
    let mut store = MockReads::new();
    store
        .expect_list()
        .times(1)
        .return_once(move |_, _, _| Ok(returned));
    service(store, identity(&reader(Role::Paralegal))).list(
        "session",
        case_id,
        MeasureDecisionReadQuery::default(),
    )
}

#[test]
fn regression_distinct_ordinary_hearing_ids_cannot_reuse_one_anchored_operation() {
    let first = crate::decision_anchor_support::ordinary_initial();
    let mut second = first.clone();
    second.snapshot.id = HearingId::from_uuid(Uuid::from_u128(999));
    crate::decision_anchor_support::refresh_ordinary(&mut second);
    assert_eq!(
        first.snapshot.receipt.operation_id,
        second.snapshot.receipt.operation_id
    );
    assert_ne!(
        first.snapshot.receipt.submission_digest,
        second.snapshot.receipt.submission_digest
    );
    assert!(list_anchors(first, second).is_err());
}

#[test]
fn distinct_ordinary_hearing_operations_and_exact_shared_details_remain_valid() {
    let first = crate::decision_anchor_support::ordinary_initial();
    assert_eq!(
        list_anchors(first.clone(), first.clone())
            .unwrap()
            .items
            .len(),
        2
    );
    let mut second = first.clone();
    second.snapshot.id = HearingId::from_uuid(Uuid::from_u128(999));
    second.snapshot.receipt.operation_id = HearingOperationId::from_uuid(Uuid::from_u128(998));
    crate::decision_anchor_support::refresh_ordinary(&mut second);
    assert_eq!(list_anchors(first, second).unwrap().items.len(), 2);
}
