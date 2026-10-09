#![allow(dead_code)]
use application::{
    case_reports::*, cases::CaseStatusFilter, identity::Principal, ApplicationError,
};
use domain::{
    case_administration::CaseAdministrativeStatus,
    cases::CaseId,
    clock::{Clock, OffsetDateTime},
    crypto::{DocumentHasher, Sha256Digest},
    identity::{Role, UserId},
    DomainError,
};
use mockall::mock;
use std::{io::Read, sync::Arc};

mock! {
    pub Store {}
    impl CaseReportStore for Store {
        fn litigators(&self, actor:&Principal, query:CaseReportLitigatorQuery, at:OffsetDateTime)->Result<CaseReportLitigatorPage,ApplicationError>;
        fn acknowledge_notice(&self, actor:&Principal, id:CaseReportId, at:OffsetDateTime)->Result<CaseReportDetail,ApplicationError>;
        fn request(&self, actor:&Principal, scope:CaseReportScope, command:CaseReportCommand, request_digest:Sha256Digest, at:OffsetDateTime)->Result<CaseReportDetail,ApplicationError>;
        fn list(&self, actor:&Principal, query:CaseReportQuery, at:OffsetDateTime)->Result<CaseReportPage,ApplicationError>;
        fn get(&self, actor:&Principal, id:CaseReportId, at:OffsetDateTime)->Result<CaseReportDetail,ApplicationError>;
        fn download(&self, actor:&Principal, id:CaseReportId, format:CaseReportFormat, at:OffsetDateTime)->Result<CaseReportDownload,ApplicationError>;
    }
}
mock! {
    pub WorkerStore {}
    impl CaseReportWorkerStore for WorkerStore {
        fn claim_next(&self, at:OffsetDateTime)->Result<Option<CaseReportClaim>,ApplicationError>;
        fn capture(&self, lease:&CaseReportLease, at:OffsetDateTime)->Result<CaseReportSnapshot,ApplicationError>;
        fn renew(&self, lease:&CaseReportLease, at:OffsetDateTime)->Result<CaseReportLease,ApplicationError>;
        fn complete(&self, lease:&CaseReportLease, snapshot:&CaseReportSnapshot, artifacts:Vec<CaseReportArtifact>, at:OffsetDateTime)->Result<CaseReportDetail,ApplicationError>;
        fn fail(&self, lease:&CaseReportLease, failure:CaseReportFailure, at:OffsetDateTime)->Result<CaseReportWorkerRun,ApplicationError>;
    }
}
mock! {
    pub Renderer {}
    impl CaseReportRenderer for Renderer {
        fn render(&self, snapshot:&CaseReportSnapshot, format:CaseReportFormat)->Result<Vec<u8>,ApplicationError>;
    }
}
pub struct TestHasher;
impl DocumentHasher for TestHasher {
    fn hash_bytes(&self, bytes: &[u8]) -> Sha256Digest {
        let mut state = [0u8; 32];
        for (index, byte) in bytes.iter().enumerate() {
            let at = index % 32;
            state[at] = state[at]
                .wrapping_mul(31)
                .wrapping_add(*byte)
                .wrapping_add(index as u8);
        }
        Sha256Digest::from_array(state)
    }
    fn hash_stream(&self, reader: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).unwrap();
        Ok(self.hash_bytes(&bytes))
    }
}
pub struct FixedClock;
impl Clock for FixedClock {
    fn now(&self) -> OffsetDateTime {
        now()
    }
}
pub fn now() -> OffsetDateTime {
    crate::case_support::instant()
}
pub fn actor(role: Role) -> Principal {
    Principal {
        id: UserId::new(),
        email: "reporter@example.test".into(),
        role,
    }
}
pub fn scope(actor: &Principal) -> CaseReportScope {
    if actor.role == Role::Owner {
        CaseReportScope::Office
    } else {
        CaseReportScope::AssignedCases
    }
}
pub fn filters() -> CaseReportFilters {
    CaseReportFilters {
        kind: CaseReportKind::CaseState,
        period_from: now() - time::Duration::days(30),
        period_before: now(),
        status: CaseStatusFilter::All,
        litigator: None,
    }
}
pub fn command() -> CaseReportCommand {
    CaseReportCommand {
        operation_id: CaseReportOperationId::new(),
        filters: filters(),
    }
}
pub fn detail(actor: &Principal, command: CaseReportCommand) -> CaseReportDetail {
    CaseReportDetail {
        id: CaseReportId::new(),
        requester: CaseReportRequester {
            principal: actor.clone(),
            account_revision: 1,
            auth_generation: 1,
        },
        scope: scope(actor),
        request_digest: case_report_request_digest(&TestHasher, actor, scope(actor), &command)
            .unwrap(),
        command,
        requested_at: now() - time::Duration::seconds(5),
        updated_at: now() - time::Duration::seconds(5),
        state: CaseReportState::Queued,
        notice: None,
    }
}
pub fn snapshot(report: &CaseReportDetail) -> CaseReportSnapshot {
    let who = if report.scope == CaseReportScope::AssignedCases {
        CaseReportLitigator {
            user_id: report.requester.principal.id,
            email: report.requester.principal.email.clone(),
        }
    } else {
        CaseReportLitigator {
            user_id: UserId::new(),
            email: "litigator@example.test".into(),
        }
    };
    let mut value = CaseReportSnapshot {
        report_id: report.id,
        requester: report.requester.clone(),
        scope: report.scope,
        filters: report.command.filters.clone(),
        checked_at: now(),
        cases: vec![CaseReportRow {
            case_id: CaseId::new(),
            title: "Current case".into(),
            reference: "REF-1".into(),
            created_at: now() - time::Duration::days(1),
            status: CaseAdministrativeStatus::Active,
            administration_revision: None,
            administration_digest: None,
            assigned_litigators: vec![who.clone()],
        }],
        workload: vec![CaseReportWorkload {
            litigator: who,
            active_cases: 1,
            closed_cases: 0,
        }],
        activity: None,
        digest: Sha256Digest::from_array([0; 32]),
    };
    value.digest = case_report_snapshot_digest(&TestHasher, &value).unwrap();
    value
}
pub fn artifact(
    report: &CaseReportDetail,
    snapshot: &CaseReportSnapshot,
    format: CaseReportFormat,
) -> CaseReportArtifact {
    let content = match format {
        CaseReportFormat::Pdf => b"%PDF-1.7\nfixture\n%%EOF\n".to_vec(),
        CaseReportFormat::Csv => b"case_id,title\r\n1,'Current case\r\n".to_vec(),
    };
    CaseReportArtifact {
        report_id: report.id,
        requester: report.requester.clone(),
        scope: report.scope,
        format,
        snapshot_digest: snapshot.digest,
        digest: TestHasher.hash_bytes(&content),
        content,
    }
}
pub fn ready(mut report: CaseReportDetail, snapshot: &CaseReportSnapshot) -> CaseReportDownload {
    let pdf = artifact(&report, snapshot, CaseReportFormat::Pdf);
    let csv = artifact(&report, snapshot, CaseReportFormat::Csv);
    report.updated_at = now();
    report.state = CaseReportState::Ready {
        snapshot_digest: snapshot.digest,
        checked_at: snapshot.checked_at,
        artifacts: vec![
            CaseReportArtifactMetadata {
                format: pdf.format,
                bytes: pdf.content.len() as u64,
                digest: pdf.digest,
            },
            CaseReportArtifactMetadata {
                format: csv.format,
                bytes: csv.content.len() as u64,
                digest: csv.digest,
            },
        ],
    };
    report.notice = Some(CaseReportNotice {
        kind: CaseReportNoticeKind::Ready,
        created_at: now(),
        read_at: None,
    });
    CaseReportDownload {
        report,
        artifact: pdf,
    }
}
pub fn identity(actor: &Principal, calls: usize) -> crate::case_support::MockIdentity {
    let value = actor.clone();
    let mut identity = crate::case_support::MockIdentity::new();
    identity
        .expect_authenticate()
        .times(calls)
        .returning(move |_| Ok(value.clone()));
    identity
}
pub fn service(store: MockStore, identity: crate::case_support::MockIdentity) -> CaseReportService {
    CaseReportService::new(
        Arc::new(store),
        Arc::new(identity),
        Arc::new(TestHasher),
        Arc::new(FixedClock),
    )
}
pub fn worker(store: MockWorkerStore, renderer: MockRenderer) -> CaseReportWorker {
    CaseReportWorker::new(
        Arc::new(store),
        Arc::new(renderer),
        Arc::new(TestHasher),
        Arc::new(FixedClock),
    )
}
pub fn claim(
    mut report: CaseReportDetail,
    snapshot: Option<CaseReportSnapshot>,
) -> CaseReportClaim {
    report.state = CaseReportState::Processing(if snapshot.is_some() {
        CaseReportPhase::Rendering
    } else {
        CaseReportPhase::Capturing
    });
    report.updated_at = now();
    CaseReportClaim {
        lease: CaseReportLease {
            report_id: report.id,
            attempt_id: CaseReportAttemptId::new(),
            token: CaseReportLeaseToken::new(),
            generation: 1,
            expires_at: now() + time::Duration::seconds(60),
        },
        report,
        snapshot,
    }
}
pub fn inconsistent<T: std::fmt::Debug>(result: Result<T, ApplicationError>) {
    assert!(
        matches!(
            result,
            Err(ApplicationError::CaseReport(
                CaseReportError::StoredInconsistent(_)
            ))
        ),
        "{result:?}"
    );
}
