use super::support::*;
use application::{case_reports::*, ApplicationError};
use quick_xml::{events::Event, Reader};

#[test]
fn pdf_keeps_spanish_nfc_nfd_and_literal_pdf_syntax_as_selectable_original_text() {
    let mut capture = snapshot(1);
    let title = "Mu\u{f1}oz: acci\u{f3}n, a\u{301}mbito, \u{bf}qu\u{e9}? \u{a1}S\u{ed}! (A) \\ <>&";
    capture.cases[0].title = title.into();
    let bytes = renderer().render(&capture, CaseReportFormat::Pdf).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
    let text = pdf_text(&bytes);
    assert!(
        compact(&text).contains(&compact(title)),
        "original Unicode must survive: {text:?}"
    );
    assert!(
        !text.contains('\u{fffd}'),
        "missing glyph replacement is not acceptable"
    );
    assert!(text.contains("Estado actual"));
    assert!(text.contains("creados"));
    assert!(text.contains("Carga"));
    assert!(
        !text.contains("'Mu"),
        "spreadsheet-neutralizing prefix must not alter the PDF"
    );
}

#[test]
fn pdf_and_csv_retain_the_same_capture_case_litigator_and_revision_identities() {
    let capture = snapshot(3);
    let engine = renderer();
    let csv = csv_records(&engine.render(&capture, CaseReportFormat::Csv).unwrap());
    let text = compact(&pdf_text(
        &engine.render(&capture, CaseReportFormat::Pdf).unwrap(),
    ));
    for identity in [
        capture.report_id.to_string(),
        capture.digest.to_string(),
        capture.requester.principal.id.to_string(),
        litigator().user_id.to_string(),
    ] {
        assert!(
            text.contains(&identity),
            "PDF omitted capture identity {identity}"
        );
        assert!(csv
            .iter()
            .any(|row| row.iter().any(|cell| cell == &identity)));
    }
    for case in &capture.cases {
        assert_eq!(text.matches(&case.case_id.to_string()).count(), 1);
        assert!(text.contains(&case.administration_digest.unwrap().to_string()));
        assert_eq!(
            csv.iter()
                .filter(|row| column(row, "case_id") == case.case_id.to_string())
                .count(),
            1
        );
    }
    assert!(text.contains("2026-09-27T12:34:56Z"));
    assert!(text.contains("2026-01-01T00:00:00Z"));
    assert!(text.contains("2026-10-01T00:00:00Z"));
}

pub(super) fn layout_within_pages(xml: &str) -> usize {
    let mut reader = Reader::from_str(xml);
    let mut size = (0.0, 0.0);
    let mut pages = 0;
    let mut words = 0;
    loop {
        match reader.read_event().unwrap() {
            Event::Start(node) if node.name().as_ref() == "page" => {
                let mut width = None;
                let mut height = None;
                for attribute in node.attributes() {
                    let attribute = attribute.unwrap();
                    let value: f64 = attribute.value.parse().unwrap();
                    match attribute.key.as_ref() {
                        "width" => width = Some(value),
                        "height" => height = Some(value),
                        _ => {}
                    }
                }
                size = (width.unwrap(), height.unwrap());
                pages += 1;
                assert!(size.0.is_finite() && size.1.is_finite() && size.0 > 0.0 && size.1 > 0.0);
            }
            Event::Start(node) if node.name().as_ref() == "word" => {
                let mut bounds = [f64::NAN; 4];
                for attribute in node.attributes() {
                    let attribute = attribute.unwrap();
                    let index = match attribute.key.as_ref() {
                        "xMin" => 0,
                        "yMin" => 1,
                        "xMax" => 2,
                        "yMax" => 3,
                        _ => continue,
                    };
                    bounds[index] = attribute.value.parse().unwrap();
                }
                assert!(bounds.iter().all(|value| value.is_finite()));
                assert!(
                    bounds[0] >= 18.0 && bounds[1] >= 18.0,
                    "text crossed page margin: {bounds:?}"
                );
                assert!(
                    bounds[2] <= size.0 - 18.0 && bounds[3] <= size.1 - 18.0,
                    "text clipped beyond page margin: {bounds:?}, {size:?}"
                );
                assert!(bounds[2] >= bounds[0] && bounds[3] >= bounds[1]);
                words += 1;
            }
            Event::Eof => break,
            _ => {}
        }
    }
    assert!(words > 0);
    pages
}

#[test]
fn pdf_paginates_without_silently_omitting_or_duplicating_any_case() {
    let capture = snapshot(180);
    let bytes = renderer().render(&capture, CaseReportFormat::Pdf).unwrap();
    let text = compact(&pdf_text(&bytes));
    assert!(layout_within_pages(&pdf_bbox(&bytes)) > 1);
    for case in &capture.cases {
        assert_eq!(
            text.matches(&case.reference).count(),
            1,
            "row must appear exactly once: {}",
            case.reference
        );
        assert_eq!(text.matches(&case.case_id.to_string()).count(), 1);
    }
    assert!(bytes.len() <= MAX_REPORT_ARTIFACT_BYTES);
}

#[test]
fn pdf_wraps_long_unbroken_fields_and_keeps_their_full_value() {
    let mut capture = snapshot(1);
    capture.cases[0].title = "W".repeat(200);
    capture.cases[0].reference = "R".repeat(100);
    let bytes = renderer().render(&capture, CaseReportFormat::Pdf).unwrap();
    let text = compact(&pdf_text(&bytes));
    assert!(text.contains(&capture.cases[0].title));
    assert!(text.contains(&capture.cases[0].reference));
    layout_within_pages(&pdf_bbox(&bytes));
}

#[test]
fn pdf_fails_explicitly_for_missing_glyphs_unsupported_scripts_and_control_characters() {
    for input in ["\u{1f980}", "\u{4e2d}", "\u{5d0}", "\u{0}", "\u{202e}abc"] {
        let mut capture = snapshot(1);
        capture.cases[0].title = input.into();
        assert!(
            matches!(
                renderer().render(&capture, CaseReportFormat::Pdf),
                Err(ApplicationError::CaseReport(CaseReportError::RenderFailed))
            ),
            "unsupported text {input:?}"
        );
    }
}
