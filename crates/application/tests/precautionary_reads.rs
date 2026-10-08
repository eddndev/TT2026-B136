#[path = "precautionary_read_support/authorization.rs"]
mod authorization_tests;
#[allow(dead_code)]
mod case_support;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_context_support/mod.rs"]
mod context_support;
#[path = "precautionary_read_support/integrity.rs"]
mod integrity_tests;
#[path = "precautionary_read_support/pagination.rs"]
mod pagination_tests;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_participant_support/mod.rs"]
mod participant_support;
mod precautionary_read_support;
#[allow(dead_code, unused_imports)]
#[path = "precautionary_receipt_support/mod.rs"]
mod receipt_support;

use precautionary_read_support::*;

#[test]
fn read_query_is_bounded_and_preserves_its_exclusive_cursor() {
    let default = PrecautionaryHearingReadQuery::default();
    assert_eq!((default.limit(), default.after_id()), (10, None));
    for limit in [1, 20] {
        let query = PrecautionaryHearingReadQuery::new(limit, Some(id(7))).unwrap();
        assert_eq!((query.limit(), query.after_id()), (limit, Some(id(7))));
    }
    for limit in [0, 21, u16::MAX] {
        assert!(PrecautionaryHearingReadQuery::new(limit, None).is_err());
    }
}

#[test]
fn staff_reads_exact_historical_capture_and_preserves_its_original_actor() {
    let saved = operation(40);
    for role in [Role::Owner, Role::Litigator, Role::Paralegal] {
        let mut reader = saved.capture.review.actor.clone();
        reader.id = UserId::from_uuid(uuid::Uuid::from_u128(999));
        reader.email = "current-reader@example.test".into();
        reader.role = role;
        for kind in READS {
            let store = successful_store(&reader, &saved, saved.clone(), kind);
            let result =
                read(&service(store, identity(&reader, 2), clock()), &saved, kind).unwrap();
            assert_eq!(result, vec![saved.clone()]);
            assert_ne!(result[0].capture.review.actor, reader);
        }
    }
}

#[allow(dead_code, unused_imports)]
#[path = "measure_decision_fixtures/mod.rs"]
mod measure_decision_fixtures;
#[allow(dead_code, unused_imports)]
#[path = "measure_source_support/mod.rs"]
mod measure_source_support;
#[path = "precautionary_read_support/newidentity_tests.rs"]
mod newidentity_tests;

#[path = "precautionary_read_support/operation_ancestry_test.rs"]
mod operation_ancestry_test;
