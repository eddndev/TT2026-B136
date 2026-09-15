mod identity;
mod metadata;
pub use identity::StubIdentity;

use application::documents::{
    CaseDocumentSummary, CaseDocumentWorkflow, CurrentDocumentMetadata, DocumentMetadata,
    DocumentOverview, DocumentPage, DocumentQuery, DocumentSummary, DocumentVersionRef,
    EvidenceExport, MetadataPage, MetadataQuery, MetadataRevision, VersionPage, VersionQuery,
};
use application::verification::{ComponentReport, ComponentStatus, Verdict, VerificationReport};
use application::ApplicationError;
use domain::audit::ChainVerification;
use domain::cases::CaseId;
use domain::crypto::{DocumentId, DocumentVersion};
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
    fn get_metadata(
        &self,
        token: &str,
        case: CaseId,
        id: DocumentId,
    ) -> Result<CurrentDocumentMetadata, ApplicationError> {
        self.check(token, case, id)?;
        Ok(Self::metadata_snapshot(3))
    }
    fn replace_metadata(
        &self,
        token: &str,
        case: CaseId,
        id: DocumentId,
        expected: MetadataRevision,
        values: DocumentMetadata,
    ) -> Result<CurrentDocumentMetadata, ApplicationError> {
        self.metadata_replace(token, case, id, expected, values)
    }
    fn metadata_history(
        &self,
        token: &str,
        case: CaseId,
        id: DocumentId,
        query: MetadataQuery,
    ) -> Result<MetadataPage, ApplicationError> {
        self.metadata_page(token, case, id, query)
    }
    fn upload_with_metadata(
        &self,
        token: &str,
        case: CaseId,
        name: &str,
        bytes: &[u8],
        values: DocumentMetadata,
    ) -> Result<DocumentOverview, ApplicationError> {
        self.metadata_upload(token, case, name, bytes, values)
    }

    fn append(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
        expected_version: DocumentVersion,
        name: &str,
        bytes: &[u8],
    ) -> Result<DocumentOverview, ApplicationError> {
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
        Ok(Self::overview(summary, 3))
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
        let metadata = Self::metadata_snapshot(3);
        let filter = query.metadata_filter();
        let matches = filter
            .document_type()
            .is_none_or(|value| metadata.values.document_type() == Some(value))
            && filter
                .classification()
                .is_none_or(|value| metadata.values.classification() == Some(value))
            && filter
                .tag()
                .is_none_or(|value| metadata.values.tags().iter().any(|tag| tag == value))
            && query.offset() == 0
            && query.sealed() != Some(true)
            && query.name().is_none_or(|name| {
                summary
                    .document
                    .name
                    .to_lowercase()
                    .contains(&name.to_lowercase())
            });
        Ok(DocumentPage {
            documents: if matches {
                vec![Self::overview(summary, 3)]
            } else {
                vec![]
            },
            has_more: false,
        })
    }

    fn get(
        &self,
        token: &str,
        case_id: CaseId,
        id: DocumentId,
    ) -> Result<DocumentOverview, ApplicationError> {
        self.check(token, case_id, id)?;
        Ok(Self::overview(Self::summary(false), 3))
    }

    fn upload(
        &self,
        token: &str,
        case_id: CaseId,
        name: &str,
        document: &[u8],
    ) -> Result<DocumentOverview, ApplicationError> {
        self.check(token, case_id, DocumentId::from_uuid(DOCUMENT_UUID))?;
        assert_eq!(name, "acta.txt");
        assert_eq!(document, b"case document");
        Ok(Self::overview(Self::summary(false), 0))
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
