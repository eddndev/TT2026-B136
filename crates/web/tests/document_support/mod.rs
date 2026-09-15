use application::documents::{
    CaseDocumentSummary, CaseDocumentWorkflow, DocumentPage, DocumentQuery, DocumentSummary,
    DocumentVersionRef, EvidenceExport, VersionPage, VersionQuery,
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
    fn append(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
        expected_version: DocumentVersion,
        name: &str,
        bytes: &[u8],
    ) -> Result<CaseDocumentSummary, ApplicationError> {
        self.check(token, case_id, id)?;
        match expected_version.get() {
            u32::MAX => return Err(ApplicationError::DocumentVersionExhausted),
            3 => {}
            _ => return Err(ApplicationError::DocumentVersionConflict),
        }
        assert_eq!(name, "revised.txt");
        assert_eq!(bytes, b"revised content");
        let mut summary = Self::summary(false);
        summary.document.version = DocumentVersion::new(4).unwrap();
        summary.document.name = name.into();
        Ok(summary)
    }

    fn history(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
        query: VersionQuery,
    ) -> Result<VersionPage, ApplicationError> {
        self.check(token, case_id, id)?;
        let mut versions = (1..=3)
            .rev()
            .filter(|version| {
                query
                    .before_version()
                    .is_none_or(|before| *version < before.get())
            })
            .map(|version| {
                let mut summary = Self::summary(version != 3);
                summary.document.version = DocumentVersion::new(version).unwrap();
                summary
            })
            .collect::<Vec<_>>();
        let has_more = versions.len() > query.limit() as usize;
        versions.truncate(query.limit() as usize);
        let next_before_version = if has_more {
            versions.last().map(|version| version.document.version)
        } else {
            None
        };
        Ok(VersionPage {
            versions,
            has_more,
            next_before_version,
            first_available_version: DocumentVersion::initial(),
        })
    }

    fn get_version(
        &self,
        token: &str,
        case_id: CaseId,
        reference: DocumentVersionRef,
    ) -> Result<CaseDocumentSummary, ApplicationError> {
        self.check(token, case_id, reference.id)?;
        if reference.version.get() > 3 {
            return Err(ApplicationError::DocumentNotFound(
                "document not found".into(),
            ));
        }
        let mut summary = Self::summary(reference.version.get() != 3);
        summary.document.version = reference.version;
        summary.document.digest_hex = format!("{:02x}", reference.version.get()).repeat(32);
        Ok(summary)
    }

    fn seal_version(
        &self,
        token: &str,
        case_id: CaseId,
        reference: DocumentVersionRef,
    ) -> Result<CaseDocumentSummary, ApplicationError> {
        let mut summary = self.get_version(token, case_id, reference)?;
        summary.document.sealed = true;
        Ok(summary)
    }

    fn verify_version(
        &self,
        token: &str,
        case_id: CaseId,
        reference: DocumentVersionRef,
    ) -> Result<VerificationReport, ApplicationError> {
        let summary = self.get_version(token, case_id, reference)?;
        let mut report = self.verify(token, case_id, reference.id)?;
        report.document_digest_hex = summary.document.digest_hex;
        Ok(report)
    }

    fn export_version(
        &self,
        token: &str,
        case_id: CaseId,
        reference: DocumentVersionRef,
    ) -> Result<EvidenceExport, ApplicationError> {
        let summary = self.get_version(token, case_id, reference)?;
        Ok(EvidenceExport {
            archive: b"historical zip bytes".to_vec(),
            file_name: "acta.txt-evidence.zip".into(),
            document_digest_hex: summary.document.digest_hex,
        })
    }

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
