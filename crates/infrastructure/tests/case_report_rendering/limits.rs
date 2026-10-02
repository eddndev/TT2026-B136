use super::support::*;
use application::{case_reports::*, ApplicationError};
use infrastructure::case_report_rendering::BoundedCaseReportRenderer;

fn capacity(capture: &CaseReportSnapshot) {
    let engine = renderer();
    for format in [CaseReportFormat::Csv, CaseReportFormat::Pdf] {
        assert!(matches!(
            engine.render(capture, format),
            Err(ApplicationError::CaseReport(
                CaseReportError::CapacityExceeded
            ))
        ));
    }
}

#[test]
fn renderer_accepts_the_complete_thousand_case_csv_without_top_n_truncation() {
    let capture = snapshot(MAX_REPORT_CASES);
    let bytes = renderer().render(&capture, CaseReportFormat::Csv).unwrap();
    let records = csv_records(&bytes);
    let cases: Vec<_> = records
        .iter()
        .filter(|row| column(row, "row_type") == "case")
        .collect();
    assert_eq!(cases.len(), MAX_REPORT_CASES);
    for (row, case) in cases.iter().zip(&capture.cases) {
        assert_eq!(column(row, "case_id"), case.case_id.to_string());
    }
    assert_eq!(column(&records[1], "total_cases"), "1000");
    assert!(bytes.len() <= MAX_REPORT_ARTIFACT_BYTES);
}

#[test]
fn renderer_rejects_rows_assignments_and_workload_over_the_declared_limits() {
    capacity(&snapshot(MAX_REPORT_CASES + 1));
    let mut assignments = snapshot(1);
    assignments.cases[0].assigned_litigators = vec![litigator(); MAX_REPORT_ASSIGNMENTS + 1];
    capacity(&assignments);
    let mut workload = snapshot(1);
    workload.workload = vec![workload.workload[0].clone(); MAX_REPORT_WORKLOAD + 1];
    capacity(&workload);
}

#[test]
fn renderer_rejects_oversized_capture_strings_before_building_a_partial_artifact() {
    let mut capture = snapshot(1);
    capture.cases[0].title = "A".repeat(MAX_REPORT_SNAPSHOT_BYTES + 1);
    capacity(&capture);
}

#[test]
fn renderer_requires_both_real_bundled_fonts_and_never_uses_a_host_fallback() {
    let regular = font("NotoSans-Regular.ttf");
    let bold = font("NotoSans-Bold.ttf");
    for (normal, strong) in [
        (&[][..], bold.as_slice()),
        (regular.as_slice(), &[][..]),
        (&regular[..32], bold.as_slice()),
        (regular.as_slice(), &bold[..32]),
    ] {
        assert!(matches!(
            BoundedCaseReportRenderer::new(normal, strong),
            Err(ApplicationError::CaseReport(
                CaseReportError::RenderUnavailable
            ))
        ));
    }
}
