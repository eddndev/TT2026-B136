use application::case_reports::CaseReportEstimate;
use application::case_reports::{estimate_report_service_ms, CaseReportTimingSample};

#[test]
fn ready_email_threshold_is_strictly_above_two_seconds() {
    assert!(!CaseReportEstimate::new(1).unwrap().requires_ready_email());
    assert!(!CaseReportEstimate::new(2_000)
        .unwrap()
        .requires_ready_email());
    assert!(CaseReportEstimate::new(2_001)
        .unwrap()
        .requires_ready_email());
}

#[test]
fn estimate_is_a_positive_bounded_duration() {
    assert!(CaseReportEstimate::new(0).is_err());
    assert!(CaseReportEstimate::new(u64::MAX).is_err());
    assert_eq!(CaseReportEstimate::new(3_250).unwrap().total_ms(), 3_250);
}

#[test]
fn service_estimate_requires_three_comparable_completed_samples() {
    let sample = |cases, ms| CaseReportTimingSample {
        cases,
        service_ms: ms,
    };
    assert_eq!(estimate_report_service_ms(80, &[]), None);
    assert_eq!(estimate_report_service_ms(80, &[sample(80, 550); 2]), None);
    assert_eq!(estimate_report_service_ms(400, &[sample(3, 90); 3]), None);
    assert_eq!(estimate_report_service_ms(80, &[sample(80, 0); 3]), None);
    assert_eq!(
        estimate_report_service_ms(80, &[sample(80, 540), sample(80, 550), sample(80, 900)]),
        Some(550)
    );
}

#[test]
fn nearby_volume_scales_measured_service_without_invented_runtime_coefficient() {
    let history = [CaseReportTimingSample {
        cases: 400,
        service_ms: 2_600,
    }; 3];
    let estimate = estimate_report_service_ms(440, &history).unwrap();
    assert!((2_850..=2_865).contains(&estimate));
    assert_eq!(estimate_report_service_ms(1000, &history), None);
}
