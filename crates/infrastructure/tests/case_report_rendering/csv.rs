use super::support::*;
use application::case_reports::*;

#[test]
fn csv_retains_an_explicit_capture_and_zero_counts_when_no_cases_match() {
    let capture = snapshot(0);
    let output = renderer().render(&capture, CaseReportFormat::Csv).unwrap();
    let records = csv_records(&output);
    assert_eq!(records.len(), 2);
    let row = &records[1];
    assert_eq!(column(row, "row_type"), "capture");
    assert_eq!(column(row, "report_id"), capture.report_id.to_string());
    assert_eq!(column(row, "snapshot_digest"), capture.digest.to_string());
    assert_eq!(column(row, "checked_at"), "2026-09-27T12:34:56Z");
    assert_eq!(column(row, "scope"), "office");
    assert_eq!(column(row, "created_from"), "2026-01-01T00:00:00Z");
    assert_eq!(column(row, "created_before"), "2026-10-01T00:00:00Z");
    for name in ["active_cases", "closed_cases", "total_cases"] {
        assert_eq!(column(row, name), "0");
    }
}

#[test]
fn csv_has_one_exact_case_row_and_one_derived_workload_row_without_ambiguous_cells() {
    let mut capture = snapshot(2);
    capture.cases[0].title = "Defensa, \"Mu\u{f1}oz\"\r\nsegunda l\u{ed}nea".into();
    capture.cases[0].reference = "REF,\"A\"".into();
    let output = renderer().render(&capture, CaseReportFormat::Csv).unwrap();
    let records = csv_records(&output);
    assert_eq!(records.len(), 5);
    assert_eq!(
        records[1..]
            .iter()
            .map(|row| column(row, "row_type"))
            .collect::<Vec<_>>(),
        ["capture", "case", "case", "workload"]
    );
    assert_eq!(column(&records[1], "total_cases"), "2");
    for (row, case) in records[2..4].iter().zip(&capture.cases) {
        assert_eq!(column(row, "report_id"), capture.report_id.to_string());
        assert_eq!(column(row, "snapshot_digest"), capture.digest.to_string());
        assert_eq!(
            column(row, "requester_id"),
            capture.requester.principal.id.to_string()
        );
        assert_eq!(column(row, "case_id"), case.case_id.to_string());
        assert_eq!(column(row, "title"), format!("'{}", case.title));
        assert_eq!(column(row, "reference"), format!("'{}", case.reference));
        assert_eq!(column(row, "status"), case.status.as_str());
        assert_eq!(column(row, "administration_revision"), "1");
        assert_eq!(
            column(row, "administration_digest"),
            case.administration_digest.unwrap().to_string()
        );
        let assignments: serde_json::Value = serde_json::from_str(
            column(row, "assigned_litigators")
                .strip_prefix('\'')
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            assignments,
            serde_json::json!([{
                "user_id": litigator().user_id.to_string(), "email": "litigator@example.test"
            }])
        );
        assert_eq!(column(row, "litigator_id"), "");
        assert_eq!(column(row, "total_cases"), "");
    }
    assert_eq!(
        column(&records[4], "litigator_id"),
        litigator().user_id.to_string()
    );
    assert_eq!(
        column(&records[4], "litigator_email"),
        "'litigator@example.test"
    );
    assert_eq!(column(&records[4], "active_cases"), "1");
    assert_eq!(column(&records[4], "closed_cases"), "1");
    assert_eq!(column(&records[4], "total_cases"), "2");
    assert_eq!(column(&records[4], "case_id"), "");
}

#[test]
fn csv_neutralizes_every_untrusted_cell_before_quoting_even_after_control_prefixes() {
    let inputs = [
        "=1+1",
        "+SUM(A1)",
        "-1+2",
        "@SUM(A1)",
        " \t=1+1",
        "\r\n=1+1",
        "\0=1+1",
        "\u{feff}=1+1",
        "\u{a0}=1+1",
        "'literal",
        "normal,\"quoted\"",
    ];
    for input in inputs {
        let mut capture = snapshot(1);
        capture.cases[0].title = input.into();
        capture.cases[0].reference = input.into();
        capture.cases[0].assigned_litigators[0].email = input.into();
        capture.workload[0].litigator.email = input.into();
        let output = renderer().render(&capture, CaseReportFormat::Csv).unwrap();
        let records = csv_records(&output);
        assert_eq!(column(&records[2], "title"), format!("'{input}"));
        assert_eq!(column(&records[2], "reference"), format!("'{input}"));
        assert_eq!(column(&records[3], "litigator_email"), format!("'{input}"));
        let assignments: serde_json::Value = serde_json::from_str(
            column(&records[2], "assigned_litigators")
                .strip_prefix('\'')
                .unwrap(),
        )
        .unwrap();
        assert_eq!(assignments[0]["email"], input);
        assert_eq!(column(&records[3], "total_cases"), "1");
    }
}
