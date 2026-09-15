use application::documents::{DocumentVersionRef, VersionQuery, VersionSelection};
use application::ApplicationError;
use domain::crypto::{DocumentId, DocumentVersion};

#[test]
fn history_queries_bound_pages_and_validate_exclusive_version_cursors() {
    for limit in [1, 100] {
        let query = VersionQuery::new(limit, Some(u32::MAX)).unwrap();
        assert_eq!(query.limit(), limit);
        assert_eq!(query.before_version().unwrap().get(), u32::MAX);
    }
    assert!(VersionQuery::new(50, None)
        .unwrap()
        .before_version()
        .is_none());
    for (limit, before) in [(0, None), (101, None), (50, Some(0))] {
        assert!(matches!(
            VersionQuery::new(limit, before),
            Err(ApplicationError::InvalidInput(_))
        ));
    }
}

#[test]
fn exact_references_keep_document_identity_and_version_separate_from_selection_policy() {
    let reference = DocumentVersionRef {
        id: DocumentId::new(),
        version: DocumentVersion::new(7).unwrap(),
    };
    let expected: domain::crypto::DocumentVersionRef = reference;
    assert_eq!(expected, reference);
    assert_ne!(VersionSelection::Current, VersionSelection::Only);
    assert_ne!(
        VersionSelection::Exact(reference.version),
        VersionSelection::Current
    );
}
