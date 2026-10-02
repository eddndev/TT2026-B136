use super::{super::protocol::*, support::*};
use application::case_reports::*;
use domain::crypto::Sha256Digest;
use uuid::Uuid;

fn context(format: CaseReportFormat) -> Context {
    let snapshot = snapshot();
    Context {
        report_id: snapshot.report_id,
        snapshot_digest: snapshot.digest,
        format,
    }
}

#[test]
fn successful_frame_is_versioned_and_binds_exact_bytes_format_report_and_capture() {
    for format in [CaseReportFormat::Pdf, CaseReportFormat::Csv] {
        let context = context(format);
        let bytes = b"literal\0report\xffbody\n";
        let expected = frame(context, bytes);
        assert_eq!(encode(context, Ok(bytes.to_vec())).unwrap(), expected);
        assert_eq!(decode(&expected, context).unwrap(), bytes);
    }
}

#[test]
fn complete_success_for_another_capture_or_format_is_not_accepted() {
    let correct = context(CaseReportFormat::Pdf);
    let wire = frame(correct, b"private report");
    for expected in [
        Context {
            report_id: CaseReportId::from_uuid(Uuid::from_u128(3)),
            ..correct
        },
        Context {
            snapshot_digest: Sha256Digest::from_array([4; 32]),
            ..correct
        },
        Context {
            format: CaseReportFormat::Csv,
            ..correct
        },
    ] {
        error(decode(&wire, expected), CaseReportError::RenderUnavailable);
    }
}

#[test]
fn wrong_lengths_hashes_magic_status_and_trailing_output_fail_closed() {
    let context = context(CaseReportFormat::Pdf);
    let valid = frame(context, b"report");
    for cut in [0, 4, 94, valid.len() - 1] {
        error(
            decode(&valid[..cut], context),
            CaseReportError::RenderUnavailable,
        );
    }
    for index in [0, 5, 6, 14, 15, 47, 63, HEADER_LEN] {
        let mut wrong = valid.clone();
        wrong[index] ^= 0x80;
        error(decode(&wrong, context), CaseReportError::RenderUnavailable);
    }
    let mut extra = valid;
    extra.push(0);
    error(decode(&extra, context), CaseReportError::RenderUnavailable);
}

#[test]
fn empty_success_is_rejected_and_announced_oversize_is_bounded_before_copying() {
    let context = context(CaseReportFormat::Csv);
    error(
        decode(&frame(context, b""), context),
        CaseReportError::RenderUnavailable,
    );
    let mut oversized = frame(context, b"x");
    oversized[7..15].copy_from_slice(&((MAX_REPORT_ARTIFACT_BYTES + 1) as u64).to_be_bytes());
    error(
        decode(&oversized, context),
        CaseReportError::CapacityExceeded,
    );
}

#[test]
fn typed_worker_failures_have_no_private_detail_or_success_metadata() {
    let context = context(CaseReportFormat::Pdf);
    for (status, expected) in [
        (1, CaseReportError::CapacityExceeded),
        (2, CaseReportError::RenderFailed),
        (3, CaseReportError::RenderUnavailable),
        (4, CaseReportError::StoredInconsistent("safe".into())),
    ] {
        let expected_kind = std::mem::discriminant(&expected);
        let wire = encode(context, Err(expected.into())).unwrap();
        let mut exact = vec![0; HEADER_LEN];
        exact[..5].copy_from_slice(b"TTRP1");
        exact[5] = status;
        exact[6] = 1;
        assert_eq!(wire, exact);
        let returned = decode(&wire, context).unwrap_err();
        assert!(!returned.to_string().contains("safe"));
        match returned {
            application::ApplicationError::CaseReport(actual) => {
                assert_eq!(std::mem::discriminant(&actual), expected_kind)
            }
            other => panic!("unexpected typed protocol error: {other}"),
        }
        let mut malformed = wire;
        malformed[15] = 1;
        error(
            decode(&malformed, context),
            CaseReportError::RenderUnavailable,
        );
    }
}
