mod case_administration_support;
#[allow(dead_code)]
mod document_store_support;

use application::cases::{CaseAdministrativeStatus, CaseRepository, CaseRevisionExpectation};
use application::documents::{
    CaseDocumentStore, DocumentAction, DocumentMetadata, DocumentRecord, MetadataRevision,
    VersionSelection,
};
use application::participants::{
    DirectoryStatus, ParticipantId, ParticipantRevision, ParticipantStore, ParticipantValues,
};
use application::ApplicationError;
use case_administration_support::Fixture;
use domain::crypto::{DocumentVersion, Sha256Digest};
use infrastructure::{
    PostgresCaseDocumentStore, PostgresCaseRepository, PostgresParticipantStore, RingSha256Hasher,
};
use std::sync::Arc;

fn values() -> ParticipantValues {
    ParticipantValues::new("Person", "Witness", None, None, DirectoryStatus::Active).unwrap()
}
#[test]
fn closing_blocks_every_mutation_preserves_missing_items_and_allows_read_and_reopening() {
    let Some(mut f) = Fixture::new() else { return };
    let cases = PostgresCaseRepository::open(&f.runtime_url, Arc::new(RingSha256Hasher)).unwrap();
    let docs = PostgresCaseDocumentStore::open(&f.runtime_url).unwrap();
    let participants =
        PostgresParticipantStore::open(&f.runtime_url, Arc::new(RingSha256Hasher)).unwrap();
    let doc = document_store_support::document();
    docs.insert(f.owner, f.case, doc.clone(), f.at).unwrap();
    let mut prepared = docs
        .load(
            f.owner,
            f.case,
            doc.id,
            VersionSelection::Only,
            DocumentAction::Seal,
        )
        .unwrap();
    prepared.seal(document_store_support::evidence()).unwrap();
    let person = ParticipantId::new();
    participants
        .create(f.owner, f.case, person, values(), f.at)
        .unwrap();
    cases
        .change_administrative_status(
            f.owner,
            f.case,
            CaseRevisionExpectation::new(0),
            CaseAdministrativeStatus::Closed,
            f.at,
        )
        .unwrap();
    let before: i64 = f
        .admin
        .query_one("SELECT count(*) FROM audit_events", &[])
        .unwrap()
        .get(0);
    let metadata = DocumentMetadata::new(None, None, &[]).unwrap();
    let version = DocumentRecord::pending(
        doc.id,
        DocumentVersion::new(2).unwrap(),
        "Next".into(),
        Sha256Digest::from_array([2; 32]),
        vec![2; 20],
    )
    .unwrap();
    for result in [
        docs.insert(f.owner, f.case, document_store_support::document(), f.at)
            .map(|_| ()),
        docs.insert_with_metadata(
            f.owner,
            f.case,
            document_store_support::document(),
            metadata.clone(),
            f.at,
        )
        .map(|_| ()),
        docs.append(f.owner, f.case, DocumentVersion::initial(), version, f.at)
            .map(|_| ()),
        docs.replace_metadata(
            f.owner,
            f.case,
            doc.id,
            MetadataRevision::new(0),
            metadata.clone(),
            f.at,
        )
        .map(|_| ()),
        docs.seal(f.owner, f.case, prepared.clone(), f.at),
        participants
            .create(f.owner, f.case, ParticipantId::new(), values(), f.at)
            .map(|_| ()),
        participants
            .replace(
                f.owner,
                f.case,
                person,
                ParticipantRevision::initial(),
                values(),
                f.at,
            )
            .map(|_| ()),
        participants
            .change_status(
                f.owner,
                f.case,
                person,
                ParticipantRevision::initial(),
                DirectoryStatus::Archived,
                f.at,
            )
            .map(|_| ()),
    ] {
        assert!(
            matches!(result, Err(ApplicationError::CaseClosed)),
            "{result:?}"
        );
    }
    assert!(matches!(
        docs.replace_metadata(
            f.owner,
            f.case,
            domain::crypto::DocumentId::new(),
            MetadataRevision::new(0),
            metadata,
            f.at
        ),
        Err(ApplicationError::DocumentNotFound(_))
    ));
    let missing = DocumentRecord::pending(
        doc.id,
        DocumentVersion::new(2).unwrap(),
        "Next".into(),
        Sha256Digest::from_array([2; 32]),
        vec![2; 20],
    )
    .unwrap();
    assert!(matches!(
        docs.seal(f.owner, f.case, missing, f.at),
        Err(ApplicationError::DocumentNotFound(_))
    ));
    assert!(matches!(
        participants.change_status(
            f.owner,
            f.case,
            ParticipantId::new(),
            ParticipantRevision::initial(),
            DirectoryStatus::Archived,
            f.at
        ),
        Err(ApplicationError::ParticipantNotFound)
    ));
    let after: i64 = f
        .admin
        .query_one("SELECT count(*) FROM audit_events", &[])
        .unwrap()
        .get(0);
    assert_eq!(after, before);
    assert_eq!(
        docs.load(
            f.owner,
            f.case,
            doc.id,
            VersionSelection::Only,
            DocumentAction::Verify
        )
        .unwrap(),
        doc
    );
    docs.record_access(f.owner, f.case, &doc, DocumentAction::Verify, f.at)
        .unwrap();
    docs.record_access(f.owner, f.case, &doc, DocumentAction::Export, f.at)
        .unwrap();
    assert_eq!(
        participants
            .get(f.owner, f.case, person, f.at)
            .unwrap()
            .values
            .directory_status(),
        DirectoryStatus::Active
    );
    cases
        .change_administrative_status(
            f.owner,
            f.case,
            CaseRevisionExpectation::new(1),
            CaseAdministrativeStatus::Active,
            f.at,
        )
        .unwrap();
    docs.seal(f.owner, f.case, prepared, f.at).unwrap();
    assert_eq!(
        participants
            .get(f.owner, f.case, person, f.at)
            .unwrap()
            .revision
            .get(),
        1
    );
}
