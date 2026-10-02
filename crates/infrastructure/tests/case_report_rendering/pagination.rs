use super::{pdf::layout_within_pages, support::*};
use application::case_reports::*;
use domain::identity::UserId;
use infrastructure::RingSha256Hasher;
use uuid::Uuid;

fn member(index: usize, wide: bool) -> CaseReportLitigator {
    let email = if wide {
        format!(
            "member{index:04}{}@{}.{}.{}.test",
            "a".repeat(54),
            "b".repeat(63),
            "c".repeat(63),
            "d".repeat(56)
        )
    } else {
        format!("member{index:04}@example.test")
    };
    CaseReportLitigator {
        user_id: UserId::from_uuid(Uuid::from_u128(10000 + index as u128)),
        email,
    }
}

fn complete(mut capture: CaseReportSnapshot) -> CaseReportSnapshot {
    capture.digest = case_report_snapshot_digest(&RingSha256Hasher, &capture).unwrap();
    validate_case_report_snapshot(&RingSha256Hasher, &capture).unwrap();
    capture
}

fn rendered_pages(capture: &CaseReportSnapshot) -> Vec<String> {
    let bytes = renderer().render(capture, CaseReportFormat::Pdf).unwrap();
    let count = layout_within_pages(&pdf_bbox(&bytes));
    let pages: Vec<_> = pdf_text(&bytes)
        .split('\u{c}')
        .filter(|page| !page.trim().is_empty())
        .map(str::to_owned)
        .collect();
    assert_eq!(pages.len(), count);
    assert!(count > 1, "fixture must cross page boundaries");
    pages
}

#[test]
fn pdf_keeps_each_short_case_heading_metadata_and_assignment_on_one_page() {
    let mut capture = snapshot(60);
    capture.workload.clear();
    for (index, case) in capture.cases.iter_mut().enumerate() {
        let who = member(index, false);
        case.assigned_litigators = vec![who.clone()];
        capture.workload.push(CaseReportWorkload {
            litigator: who,
            active_cases: u64::from(index % 2 == 0),
            closed_cases: u64::from(index % 2 != 0),
        });
    }
    let capture = complete(capture);
    let pages = rendered_pages(&capture);
    let text = compact(&pages.join("\n"));
    for (index, case) in capture.cases.iter().enumerate() {
        let id = case.case_id.to_string();
        assert_eq!(text.matches(&id).count(), 1);
        let page = compact(pages.iter().find(|page| page.contains(&id)).unwrap());
        let state = if index % 2 == 0 { "Activo" } else { "Cerrado" };
        for value in [
            format!("Expediente{}|{state}", index + 1),
            compact(&case.title),
            compact(&format!("Referencia: {}", case.reference)),
            compact(&format!(
                "Litigante: {} | {}",
                case.assigned_litigators[0].email, case.assigned_litigators[0].user_id
            )),
        ] {
            assert!(
                page.contains(&value),
                "case {id} was split across pages: {value}"
            );
        }
    }
}

#[test]
fn pdf_keeps_a_wrapped_workload_record_with_its_heading_and_counts() {
    let mut capture = snapshot(1);
    capture.cases[0].assigned_litigators = (0..60).map(|index| member(index, true)).collect();
    capture.workload = capture.cases[0]
        .assigned_litigators
        .iter()
        .cloned()
        .map(|litigator| CaseReportWorkload {
            litigator,
            active_cases: 1,
            closed_cases: 0,
        })
        .collect();
    let capture = complete(capture);
    let pages = rendered_pages(&capture);
    for (index, row) in capture.workload.iter().enumerate() {
        let heading = format!("Litigante {}", index + 1);
        let (page, start) = pages
            .iter()
            .find_map(|page| {
                page.lines()
                    .position(|line| line.trim() == heading)
                    .map(|start| (page, start))
            })
            .unwrap();
        let block = page
            .lines()
            .skip(start + 1)
            .take_while(|line| !line.trim().starts_with("Litigante "))
            .collect::<Vec<_>>()
            .join("\n");
        let block = compact(&block);
        for value in [
            row.litigator.email.clone(),
            row.litigator.user_id.to_string(),
            "Activos:1Cerrados:0Total:1".into(),
        ] {
            assert!(
                block.contains(&value),
                "workload {heading} was split: {value}"
            );
        }
    }
}

#[test]
fn pdf_labels_long_case_continuations_without_repeating_case_identity_or_omitting_assignments() {
    let mut capture = snapshot(1);
    capture.cases[0].assigned_litigators = (0..120).map(|index| member(index, false)).collect();
    capture.workload = capture.cases[0]
        .assigned_litigators
        .iter()
        .cloned()
        .map(|litigator| CaseReportWorkload {
            litigator,
            active_cases: 1,
            closed_cases: 0,
        })
        .collect();
    let capture = complete(capture);
    let pages = rendered_pages(&capture);
    let case = &capture.cases[0];
    let first_page = pages
        .iter()
        .position(|page| page.contains(&case.case_id.to_string()))
        .unwrap();
    let text = compact(&pages.join("\n"));
    assert_eq!(text.matches(&case.case_id.to_string()).count(), 1);
    assert_eq!(text.matches(&case.reference).count(), 1);
    let mut continuation_pages = 0;
    for (index, page) in pages.iter().enumerate() {
        let page = compact(page);
        let Some(first_assignment) = page.find("Litigante:") else {
            continue;
        };
        if index > first_page {
            let heading = page
                .find("Expediente1|Activo(continuacion)")
                .expect("assignment continuation must identify its case before the first line");
            assert!(heading < first_assignment);
            continuation_pages += 1;
        }
    }
    assert!(
        continuation_pages >= 2,
        "fixture must require multiple continuations"
    );
    for who in &case.assigned_litigators {
        let value = compact(&format!("Litigante: {} | {}", who.email, who.user_id));
        assert_eq!(
            text.matches(&value).count(),
            1,
            "assignment omitted or duplicated: {value}"
        );
    }
}
