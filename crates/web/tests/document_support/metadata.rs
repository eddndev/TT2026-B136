use application::documents::{
    CaseDocumentSummary, CurrentDocumentMetadata, DocumentMetadata, DocumentMetadataRevision,
    DocumentOverview, MetadataActorSnapshot, MetadataPage, MetadataQuery, MetadataRevision,
};
use application::ApplicationError;
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::{DocumentId, Sha256Digest},
    identity::UserId,
};

use super::{StubWorkflow, DOCUMENT_UUID};

impl StubWorkflow {
    pub(super) fn metadata_snapshot(revision: u32) -> CurrentDocumentMetadata {
        CurrentDocumentMetadata {
            metadata_revision: MetadataRevision::new(revision),
            values: if revision == 0 {
                DocumentMetadata::empty()
            } else {
                DocumentMetadata::new(
                    Some("Escrito"),
                    Some("Penal"),
                    &["a,b".into(), "acci\u{00f3}n".into(), "z".into()],
                )
                .unwrap()
            },
        }
    }

    pub(super) fn overview(content: CaseDocumentSummary, revision: u32) -> DocumentOverview {
        DocumentOverview {
            content,
            current_metadata: Self::metadata_snapshot(revision),
        }
    }

    pub(super) fn metadata_replace(
        &self,
        token: &str,
        case: CaseId,
        id: DocumentId,
        expected: MetadataRevision,
        values: DocumentMetadata,
    ) -> Result<CurrentDocumentMetadata, ApplicationError> {
        self.check(token, case, id)?;
        match expected.get() {
            u32::MAX => return Err(ApplicationError::DocumentMetadataRevisionExhausted),
            3 => {}
            _ => return Err(ApplicationError::DocumentMetadataConflict),
        }
        Ok(CurrentDocumentMetadata {
            metadata_revision: MetadataRevision::new(4),
            values,
        })
    }

    pub(super) fn metadata_page(
        &self,
        token: &str,
        case: CaseId,
        id: DocumentId,
        query: MetadataQuery,
    ) -> Result<MetadataPage, ApplicationError> {
        self.check(token, case, id)?;
        let mut revisions = (1..=3)
            .rev()
            .filter(|revision| {
                query
                    .before_revision()
                    .is_none_or(|before| *revision < before.get())
            })
            .map(|revision| {
                let current = Self::metadata_snapshot(revision);
                DocumentMetadataRevision {
                    metadata_revision: current.metadata_revision,
                    values: current.values,
                    metadata_digest: Sha256Digest::from_array([3; 32]),
                    changed_at: OffsetDateTime::from_unix_timestamp(1_735_689_600).unwrap(),
                    changed_by: MetadataActorSnapshot {
                        id: UserId::from_uuid(uuid::Uuid::from_u128(3)),
                        email: "historical@example.com".into(),
                    },
                }
            })
            .collect::<Vec<_>>();
        let has_more = revisions.len() > query.limit() as usize;
        revisions.truncate(query.limit() as usize);
        let next_before_revision = if has_more {
            revisions.last().map(|revision| revision.metadata_revision)
        } else {
            None
        };
        Ok(MetadataPage {
            revisions,
            has_more,
            next_before_revision,
        })
    }

    pub(super) fn metadata_upload(
        &self,
        token: &str,
        case: CaseId,
        name: &str,
        bytes: &[u8],
        values: DocumentMetadata,
    ) -> Result<DocumentOverview, ApplicationError> {
        self.check(token, case, DocumentId::from_uuid(DOCUMENT_UUID))?;
        assert_eq!(name, "acta.txt");
        if bytes.len() == 16 * 1024 * 1024 {
            assert!(bytes.iter().all(|byte| *byte == 42));
        } else {
            assert_eq!(bytes, b"case document");
        }
        Ok(DocumentOverview {
            content: Self::summary(false),
            current_metadata: CurrentDocumentMetadata {
                metadata_revision: MetadataRevision::new(1),
                values,
            },
        })
    }
}
