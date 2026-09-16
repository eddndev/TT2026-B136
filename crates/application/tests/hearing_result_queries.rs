use application::hearing_results::{
    HearingResultHistoryQuery, HearingResultQuery, HearingResultStatusFilter,
};
use domain::hearing_results::{HearingResultId, HearingResultStatus};

#[test]
fn root_listing_keeps_explicit_status_and_bounded_uuid_cursor() {
    let id = HearingResultId::new();
    for limit in [0, 101, u32::MAX] {
        assert!(HearingResultQuery::new(limit, Some(id), HearingResultStatusFilter::All).is_err());
    }
    for limit in [1, 20, 100] {
        let query =
            HearingResultQuery::new(limit, Some(id), HearingResultStatusFilter::All).unwrap();
        assert_eq!(query.limit(), limit);
        assert_eq!(query.after_id(), Some(id));
        assert_eq!(query.status().status(), None);
    }
    assert_eq!(
        HearingResultStatusFilter::default(),
        HearingResultStatusFilter::All
    );
    assert_eq!(
        HearingResultStatusFilter::Recorded.status(),
        Some(HearingResultStatus::Recorded)
    );
    assert_eq!(
        HearingResultStatusFilter::Withdrawn.status(),
        Some(HearingResultStatus::Withdrawn)
    );
}

#[test]
fn history_limit_is_smaller_than_root_listing_and_revision_is_positive() {
    for limit in [0, 21, 100, u32::MAX] {
        assert!(HearingResultHistoryQuery::new(limit, None).is_err());
    }
    for limit in [1, 10, 20] {
        let query = HearingResultHistoryQuery::new(limit, Some(1)).unwrap();
        assert_eq!(query.limit(), limit);
        assert_eq!(query.before_revision().unwrap().get(), 1);
    }
    assert!(HearingResultHistoryQuery::new(10, Some(0)).is_err());
}
