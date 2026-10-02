use application::{
    case_reports::*, cases::CaseStatusFilter, identity::Principal, ApplicationError,
};
use domain::{
    crypto::Sha256Digest,
    identity::{Role, UserId},
};
use time::OffsetDateTime;
use uuid::Uuid;

pub fn snapshot() -> CaseReportSnapshot {
    let at = OffsetDateTime::from_unix_timestamp(1_735_689_600).unwrap();
    let mut value = CaseReportSnapshot {
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
            created_from: at,
            created_before: at + time::Duration::days(1),
            status: CaseStatusFilter::All,
            assigned_litigator: None,
        },
        checked_at: at,
        cases: Vec::new(),
        workload: Vec::new(),
        digest: Sha256Digest::from_array([0; 32]),
    };
    value.digest = case_report_snapshot_digest(&crate::RingSha256Hasher, &value).unwrap();
    value
}
pub fn error<T: std::fmt::Debug>(result: Result<T, ApplicationError>, expected: CaseReportError) {
    match result.unwrap_err() {
        ApplicationError::CaseReport(actual) => assert_eq!(
            std::mem::discriminant(&actual),
            std::mem::discriminant(&expected)
        ),
        other => panic!("unexpected isolated renderer error: {other}"),
    }
}

pub fn frame(context: super::super::protocol::Context, content: &[u8]) -> Vec<u8> {
    use domain::crypto::DocumentHasher;
    let mut wire = b"TTRP1".to_vec();
    wire.push(0);
    wire.push(match context.format {
        CaseReportFormat::Pdf => 1,
        CaseReportFormat::Csv => 2,
    });
    wire.extend_from_slice(&(content.len() as u64).to_be_bytes());
    wire.extend_from_slice(crate::RingSha256Hasher.hash_bytes(content).as_bytes());
    wire.extend_from_slice(context.report_id.as_uuid().as_bytes());
    wire.extend_from_slice(context.snapshot_digest.as_bytes());
    assert_eq!(wire.len(), 95);
    wire.extend_from_slice(content);
    wire
}
