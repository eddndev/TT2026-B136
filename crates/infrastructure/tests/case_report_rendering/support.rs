use application::{case_reports::*, cases::CaseStatusFilter, identity::Principal};
use domain::{
    case_administration::{CaseAdministrativeStatus, CaseRevision},
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::Sha256Digest,
    identity::{Role, UserId},
};
use infrastructure::case_report_rendering::BoundedCaseReportRenderer;
use std::{fs, path::PathBuf, process::Command};
use uuid::Uuid;

pub const HEADER: &[&str] = &[
    "row_type",
    "report_id",
    "snapshot_digest",
    "checked_at",
    "requester_id",
    "scope",
    "created_from",
    "created_before",
    "status_filter",
    "assigned_litigator_filter",
    "case_id",
    "title",
    "reference",
    "created_at",
    "status",
    "administration_revision",
    "administration_digest",
    "assigned_litigators",
    "litigator_id",
    "litigator_email",
    "active_cases",
    "closed_cases",
    "total_cases",
];

pub fn font(name: &str) -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets/case-reports")
            .join(name),
    )
    .expect("the pinned report font must be vendored; host fallback is forbidden")
}

pub fn renderer() -> BoundedCaseReportRenderer {
    BoundedCaseReportRenderer::new(&font("NotoSans-Regular.ttf"), &font("NotoSans-Bold.ttf"))
        .expect("the pinned report fonts must initialize")
}

pub fn instant(value: &str) -> OffsetDateTime {
    OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339).unwrap()
}

pub fn litigator() -> CaseReportLitigator {
    CaseReportLitigator {
        user_id: UserId::from_uuid(Uuid::from_u128(10)),
        email: "litigator@example.test".into(),
    }
}

pub fn snapshot(count: usize) -> CaseReportSnapshot {
    let who = litigator();
    let cases = (0..count)
        .map(|index| CaseReportRow {
            case_id: CaseId::from_uuid(Uuid::from_u128(1000 + index as u128)),
            title: format!("Expediente {index:04}"),
            reference: format!("REF{index:04}END"),
            created_at: instant("2026-02-01T12:00:00Z"),
            status: if index % 2 == 0 {
                CaseAdministrativeStatus::Active
            } else {
                CaseAdministrativeStatus::Closed
            },
            administration_revision: Some(CaseRevision::FIRST),
            administration_digest: Some(Sha256Digest::from_array([3; 32])),
            assigned_litigators: vec![who.clone()],
        })
        .collect();
    CaseReportSnapshot {
        activity: None,
        report_id: CaseReportId::from_uuid(Uuid::from_u128(1)),
        requester: CaseReportRequester {
            principal: Principal {
                id: UserId::from_uuid(Uuid::from_u128(2)),
                email: "owner@example.test".into(),
                role: Role::Owner,
            },
            account_revision: 4,
            auth_generation: 2,
        },
        scope: CaseReportScope::Office,
        filters: CaseReportFilters {
            kind: CaseReportKind::CaseState,
            period_from: instant("2026-01-01T00:00:00Z"),
            period_before: instant("2026-10-01T00:00:00Z"),
            status: CaseStatusFilter::All,
            litigator: None,
        },
        checked_at: instant("2026-09-27T12:34:56Z"),
        cases,
        workload: if count == 0 {
            vec![]
        } else {
            vec![CaseReportWorkload {
                litigator: who,
                active_cases: count.div_ceil(2) as u64,
                closed_cases: (count / 2) as u64,
            }]
        },
        digest: Sha256Digest::from_array([7; 32]),
    }
}

pub fn column<'a>(row: &'a [String], name: &str) -> &'a str {
    &row[HEADER.iter().position(|value| *value == name).unwrap()]
}

/// Independent strict RFC 4180 reader for the renderer's fixed-column output.
pub fn csv_records(bytes: &[u8]) -> Vec<Vec<String>> {
    let text = std::str::from_utf8(bytes).expect("CSV must be UTF-8");
    assert!(
        !text.starts_with('\u{feff}'),
        "the machine-readable CSV has no BOM"
    );
    let mut chars = text.chars().peekable();
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut closed = false;
    while let Some(ch) = chars.next() {
        if quoted {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    cell.push('"');
                } else {
                    quoted = false;
                    closed = true;
                }
            } else {
                cell.push(ch);
            }
            continue;
        }
        match ch {
            '"' if cell.is_empty() && !closed => quoted = true,
            ',' => {
                record.push(std::mem::take(&mut cell));
                closed = false;
            }
            '\r' => {
                assert_eq!(chars.next(), Some('\n'), "records require CRLF");
                record.push(std::mem::take(&mut cell));
                records.push(std::mem::take(&mut record));
                closed = false;
            }
            '\n' | '"' => panic!("unescaped CSV separator or quote"),
            _ => {
                assert!(!closed, "trailing data after quoted cell");
                cell.push(ch);
            }
        }
    }
    assert!(
        !quoted && cell.is_empty() && record.is_empty(),
        "complete CRLF-terminated records required"
    );
    assert_eq!(records.first().unwrap(), HEADER);
    assert!(records.iter().all(|row| row.len() == HEADER.len()));
    records
}

pub fn pdf_text(bytes: &[u8]) -> String {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("report.pdf");
    fs::write(&path, bytes).unwrap();
    let output = Command::new("pdftotext")
        .args(["-enc", "UTF-8", "-raw"])
        .arg(path)
        .arg("-")
        .output()
        .expect("Poppler pdftotext is required for PDF verification");
    assert!(
        output.status.success(),
        "pdftotext: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("selectable report text must be UTF-8")
}

pub fn pdf_bbox(bytes: &[u8]) -> String {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("report.pdf");
    fs::write(&path, bytes).unwrap();
    let output = Command::new("pdftotext")
        .args(["-enc", "UTF-8", "-bbox-layout"])
        .arg(path)
        .arg("-")
        .output()
        .expect("Poppler pdftotext is required for layout verification");
    assert!(
        output.status.success(),
        "pdftotext: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

pub fn compact(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}
