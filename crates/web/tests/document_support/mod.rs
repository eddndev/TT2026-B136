use application::documents::{
    CaseDocumentSummary, CaseDocumentWorkflow, DocumentPage, DocumentQuery, DocumentSummary,
    EvidenceExport,
};
use application::identity::{
    EnrollmentResult, IdentityWorkflow, LoginChallenge, Principal, SessionResult,
};
use application::verification::{ComponentReport, ComponentStatus, Verdict, VerificationReport};
use application::ApplicationError;
use domain::audit::ChainVerification;
use domain::cases::CaseId;
use domain::crypto::{DocumentId, DocumentVersion};
use domain::identity::{Permission, Role};
use std::sync::atomic::{AtomicUsize, Ordering};
use uuid::Uuid;

pub const DOCUMENT_UUID: Uuid = Uuid::from_u128(0x00112233_4455_6677_8899_aabbccddeeff);
pub const CASE_UUID: Uuid = Uuid::from_u128(0x11223344_5566_7788_99aa_bbccddeeff00);

#[derive(Default)]
pub struct StubWorkflow {
    pub calls: AtomicUsize,
}

impl StubWorkflow {
    fn check(&self, token: &str, case_id: CaseId, id: DocumentId) -> Result<(), ApplicationError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        match token {
            "owner-token" => {}
            "client-token" => return Err(ApplicationError::PermissionDenied),
            _ => return Err(ApplicationError::InvalidSession),
        }
        if case_id.as_uuid() != CASE_UUID || id.as_uuid() != DOCUMENT_UUID {
            return Err(ApplicationError::DocumentNotFound(
                "document not found".into(),
            ));
        }
        Ok(())
    }
    fn summary(sealed: bool) -> CaseDocumentSummary {
        CaseDocumentSummary {
            case_id: CaseId::from_uuid(CASE_UUID),
            document: DocumentSummary {
                id: DocumentId::from_uuid(DOCUMENT_UUID),
                version: DocumentVersion::initial(),
                name: "acta.txt".into(),
                digest_hex: "ab".repeat(32),
                sealed,
            },
        }
    }
}

impl CaseDocumentWorkflow for StubWorkflow {
    fn list(
        &self,
        token: &str,
        case_id: CaseId,
        query: DocumentQuery,
    ) -> Result<DocumentPage, ApplicationError> {
        self.check(token, case_id, DocumentId::from_uuid(DOCUMENT_UUID))?;
        let summary = Self::summary(false);
        let matches = query.offset() == 0
            && query.sealed() != Some(true)
            && query.name().is_none_or(|name| {
                summary
                    .document
                    .name
                    .to_lowercase()
                    .contains(&name.to_lowercase())
            });
        Ok(DocumentPage {
            documents: if matches { vec![summary] } else { vec![] },
            has_more: false,
        })
    }

    fn get(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
    ) -> Result<CaseDocumentSummary, ApplicationError> {
        self.check(token, case_id, id)?;
        Ok(Self::summary(false))
    }

    fn upload(
        &self,
        token: &str,
        case_id: CaseId,
        name: &str,
        document: &[u8],
    ) -> Result<CaseDocumentSummary, ApplicationError> {
        self.check(token, case_id, DocumentId::from_uuid(DOCUMENT_UUID))?;
        assert_eq!(name, "acta.txt");
        assert_eq!(document, b"case document");
        Ok(Self::summary(false))
    }
    fn seal(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
    ) -> Result<CaseDocumentSummary, ApplicationError> {
        self.check(token, case_id, id)?;
        Ok(Self::summary(true))
    }
    fn verify(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
    ) -> Result<VerificationReport, ApplicationError> {
        self.check(token, case_id, id)?;
        let passed = || ComponentReport {
            status: ComponentStatus::Passed,
            detail: "accepted".into(),
        };
        Ok(VerificationReport {
            document_digest_hex: "ab".repeat(32),
            integrity: passed(),
            signature: passed(),
            certificate: passed(),
            timestamp: passed(),
            verdict: Verdict::Valid,
        })
    }
    fn export_evidence(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
    ) -> Result<EvidenceExport, ApplicationError> {
        self.check(token, case_id, id)?;
        Ok(EvidenceExport {
            archive: b"zip bytes".to_vec(),
            file_name: "acta.txt-evidence.zip".into(),
            document_digest_hex: "ab".repeat(32),
        })
    }
    fn verify_audit(&self, token: &str) -> Result<ChainVerification, ApplicationError> {
        self.check(
            token,
            CaseId::from_uuid(CASE_UUID),
            DocumentId::from_uuid(DOCUMENT_UUID),
        )?;
        Ok(ChainVerification::Valid { entries: 4 })
    }
}

pub struct StubIdentity;

impl IdentityWorkflow for StubIdentity {
    fn bootstrap_owner(
        &self,
        _email: &str,
        _password: &str,
    ) -> Result<EnrollmentResult, ApplicationError> {
        Err(ApplicationError::BootstrapClosed)
    }

    fn create_user(
        &self,
        _token: &str,
        _email: &str,
        _password: &str,
        _role: Role,
    ) -> Result<EnrollmentResult, ApplicationError> {
        Err(ApplicationError::InvalidInput("unused".to_string()))
    }

    fn start_login(
        &self,
        _email: &str,
        _password: &str,
    ) -> Result<LoginChallenge, ApplicationError> {
        Err(ApplicationError::InvalidCredentials)
    }

    fn complete_totp(
        &self,
        _challenge_token: &str,
        _code: &str,
    ) -> Result<SessionResult, ApplicationError> {
        Err(ApplicationError::MfaRejected)
    }

    fn complete_recovery(
        &self,
        _challenge_token: &str,
        _code: &str,
    ) -> Result<SessionResult, ApplicationError> {
        Err(ApplicationError::MfaRejected)
    }

    fn authenticate(&self, token: &str) -> Result<Principal, ApplicationError> {
        panic!("document authentication must run in the workflow: {token}")
    }

    fn authorize(
        &self,
        token: &str,
        permission: Permission,
    ) -> Result<Principal, ApplicationError> {
        let principal = self.authenticate(token)?;
        if principal.role.allows(permission) {
            Ok(principal)
        } else {
            Err(ApplicationError::PermissionDenied)
        }
    }

    fn logout(&self, _access_token: &str) -> Result<(), ApplicationError> {
        Ok(())
    }
}
