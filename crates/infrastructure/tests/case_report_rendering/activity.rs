use super::support::*;
use application::case_reports::*;
use application::ApplicationError;

fn activity(complete: bool) -> CaseReportSnapshot {
    let mut value = snapshot(2);
    value.filters.kind = CaseReportKind::LitigatorActivity;
    value.filters.period_from = instant("2026-09-01T00:00:00Z");
    value.workload.clear();
    value.activity = Some(CaseReportActivitySnapshot {
        actors: vec![litigator()],
        rows: value
            .cases
            .iter()
            .enumerate()
            .map(|(index, case)| CaseReportActivityRow {
                case_id: case.case_id,
                litigator_id: litigator().user_id,
                documents_uploaded: 2 + index as u64,
                procedural_activities: 5 + index as u64,
                deadlines_attended: 7 + index as u64,
            })
            .collect(),
        documents_complete: complete,
    });
    value
}

#[test]
fn both_formats_report_the_same_author_counts_and_capture() {
    let snapshot = activity(true);
    let render = renderer();
    let csv = String::from_utf8(render.render(&snapshot, CaseReportFormat::Csv).unwrap()).unwrap();
    let lines: Vec<_> = csv.lines().collect();
    let headers: Vec<_> = lines[0].split(',').collect();
    let total: Vec<_> = lines
        .iter()
        .find(|line| line.starts_with("litigator,"))
        .unwrap()
        .split(',')
        .collect();
    let cell = |name: &str| total[headers.iter().position(|h| *h == name).unwrap()];
    assert_eq!(cell("documents_uploaded"), "5");
    assert_eq!(cell("procedural_activities"), "11");
    assert_eq!(cell("deadlines_attended"), "15");
    assert_eq!(cell("documents_complete"), "true");
    assert_eq!(cell("litigator_id"), litigator().user_id.to_string());
    assert_eq!(cell("snapshot_digest"), snapshot.digest.to_string());
    let text = pdf_text(&render.render(&snapshot, CaseReportFormat::Pdf).unwrap());
    for expected in [
        "Actividad registrada",
        "Documentos: 5",
        "Actuaciones: 11",
        "Plazos atendidos: 15",
        "litigator@example.test",
    ] {
        assert!(text.contains(expected), "missing {expected}");
    }
    assert!(compact(&text).contains(&snapshot.digest.to_string()));
    assert!(!text.contains("Carga por litigante"));
}

#[test]
fn missing_historical_authorship_is_visible_in_both_formats() {
    let snapshot = activity(false);
    let render = renderer();
    let csv = String::from_utf8(render.render(&snapshot, CaseReportFormat::Csv).unwrap()).unwrap();
    let headers: Vec<_> = csv.lines().next().unwrap().split(',').collect();
    let column = headers
        .iter()
        .position(|h| *h == "documents_complete")
        .unwrap();
    for line in csv.lines().skip(1) {
        assert_eq!(line.split(',').nth(column), Some("false"));
    }
    let text = pdf_text(&render.render(&snapshot, CaseReportFormat::Pdf).unwrap());
    assert!(text.contains("Cobertura documental incompleta"));
    assert!(text.contains("identificados"));
}

#[test]
fn activity_csv_retains_every_case_author_row_and_neutralizes_author_text() {
    let mut snapshot = activity(true);
    snapshot.activity.as_mut().unwrap().actors[0].email = "=1+1".into();
    let csv =
        String::from_utf8(renderer().render(&snapshot, CaseReportFormat::Csv).unwrap()).unwrap();
    let headers: Vec<_> = csv.lines().next().unwrap().split(',').collect();
    let cell = |row: &str, name: &str| -> String {
        row.split(',')
            .nth(headers.iter().position(|h| *h == name).unwrap())
            .unwrap()
            .into()
    };
    let rows: Vec<_> = csv
        .lines()
        .filter(|line| line.starts_with("case_activity,"))
        .collect();
    let expected = &snapshot.activity.as_ref().unwrap().rows;
    assert_eq!(rows.len(), expected.len());
    for (row, captured) in rows.iter().zip(expected) {
        assert_eq!(cell(row, "case_id"), captured.case_id.to_string());
        assert_eq!(cell(row, "litigator_id"), captured.litigator_id.to_string());
        assert_eq!(cell(row, "litigator_email"), "'=1+1");
        assert_eq!(
            cell(row, "documents_uploaded"),
            captured.documents_uploaded.to_string()
        );
        assert_eq!(
            cell(row, "procedural_activities"),
            captured.procedural_activities.to_string()
        );
        assert_eq!(
            cell(row, "deadlines_attended"),
            captured.deadlines_attended.to_string()
        );
    }
    let totals = csv
        .lines()
        .find(|line| line.starts_with("litigator,"))
        .unwrap();
    assert_eq!(cell(totals, "litigator_email"), "'=1+1");
}

#[test]
fn activity_rejects_excessive_actors_rows_text_and_counter_sums_before_export() {
    let render = renderer();
    for dimension in 0..6 {
        let mut snapshot = activity(true);
        let payload = snapshot.activity.as_mut().unwrap();
        match dimension {
            0 => payload.actors = vec![litigator(); MAX_REPORT_WORKLOAD + 1],
            1 => payload.rows = vec![payload.rows[0].clone(); MAX_REPORT_ASSIGNMENTS + 1],
            2 => payload.actors[0].email = "A".repeat(MAX_REPORT_SNAPSHOT_BYTES + 1),
            3 => payload.rows[0].documents_uploaded = u64::MAX,
            4 => payload.rows[0].procedural_activities = u64::MAX,
            _ => payload.rows[0].deadlines_attended = u64::MAX,
        }
        for format in [CaseReportFormat::Csv, CaseReportFormat::Pdf] {
            assert!(
                matches!(
                    render.render(&snapshot, format),
                    Err(ApplicationError::CaseReport(
                        CaseReportError::CapacityExceeded
                    ))
                ),
                "dimension {dimension}, format {format:?}"
            );
        }
    }
}
