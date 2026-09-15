use std::io::Read;

use application::documents::{
    metadata_digest, DocumentAction, DocumentMetadata, DocumentMetadataFilter, DocumentQuery,
    MetadataQuery, MetadataRevision,
};
use application::ApplicationError;
use domain::crypto::{DocumentHasher, Sha256Digest};
use domain::identity::Permission;
use domain::DomainError;

#[test]
fn metadata_query_requires_bounded_page_and_positive_exclusive_cursor() {
    for (limit, before) in [(0, None), (101, None), (1, Some(0))] {
        assert!(matches!(
            MetadataQuery::new(limit, before),
            Err(ApplicationError::InvalidInput(_))
        ));
    }
    for (limit, before) in [(1, None), (100, Some(1)), (2, Some(u32::MAX))] {
        let query = MetadataQuery::new(limit, before).unwrap();
        assert_eq!(query.limit(), limit);
        assert_eq!(query.before_revision().map(MetadataRevision::get), before);
    }
}

#[test]
fn filters_preserve_literals_and_query_pagination() {
    let filter =
        DocumentMetadataFilter::new(Some("  Escrito "), Some("Civil"), Some(" a,b%_ ")).unwrap();
    assert_eq!(filter.document_type(), Some("Escrito"));
    assert_eq!(filter.classification(), Some("Civil"));
    assert_eq!(filter.tag(), Some("a,b%_"));
    let query = DocumentQuery::new(4, 8, Some("proof"), Some(true)).unwrap();
    assert_eq!(query.metadata_filter(), &DocumentMetadataFilter::default());
    let filtered = query.with_metadata_filter(filter.clone());
    assert_eq!(filtered.metadata_filter(), &filter);
    assert_eq!(
        (
            filtered.limit(),
            filtered.offset(),
            filtered.name(),
            filtered.sealed()
        ),
        (4, 8, Some("proof"), Some(true))
    );
    let blank = DocumentMetadataFilter::new(Some(" "), Some("\u{2003}"), None).unwrap();
    assert_eq!(blank, DocumentMetadataFilter::default());
}

#[test]
fn filters_reject_original_controls_empty_tag_and_overlong_scalars() {
    for (kind, class, tag) in [
        (Some("\n"), None, None),
        (None, Some("Civil\t"), None),
        (None, None, Some(" a\u{85}")),
        (None, None, Some(" ")),
    ] {
        assert!(matches!(
            DocumentMetadataFilter::new(kind, class, tag),
            Err(ApplicationError::InvalidInput(_))
        ));
    }
    let kind = "\u{e1}".repeat(80);
    let tag = "\u{e1}".repeat(40);
    assert!(DocumentMetadataFilter::new(Some(&kind), Some(&kind), Some(&tag)).is_ok());
    assert!(DocumentMetadataFilter::new(Some(&(kind.clone() + "a")), None, None).is_err());
    assert!(DocumentMetadataFilter::new(None, Some(&(kind + "a")), None).is_err());
    assert!(DocumentMetadataFilter::new(None, None, Some(&(tag + "a"))).is_err());
}

#[test]
fn metadata_actions_have_explicit_permission_and_event_names() {
    for (action, permission, event) in [
        (
            DocumentAction::Classify,
            Permission::ClassifyDocument,
            "document.metadata_changed",
        ),
        (
            DocumentAction::ReadMetadata,
            Permission::ReadDocument,
            "document.metadata_read",
        ),
        (
            DocumentAction::MetadataHistory,
            Permission::ReadDocument,
            "document.metadata_history_listed",
        ),
    ] {
        assert_eq!(action.permission(), permission);
        assert_eq!(action.audit_action(), event);
    }
}

struct CanonicalHasher;
impl DocumentHasher for CanonicalHasher {
    fn hash_bytes(&self, bytes: &[u8]) -> Sha256Digest {
        assert_eq!(bytes, b"DMETA1\0\0\0\0\0\0");
        Sha256Digest::from_hex("adbad13daff0fc70b3309e1e58a20aecadc00077dac6f2a04349c4001e8443bb")
            .unwrap()
    }
    fn hash_stream(&self, _: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        panic!("bounded metadata must use hash_bytes")
    }
}

#[test]
fn digest_helper_passes_canonical_bytes_to_the_hasher_port() {
    let digest = metadata_digest(&CanonicalHasher, &DocumentMetadata::empty());
    assert_eq!(
        digest.to_hex(),
        "adbad13daff0fc70b3309e1e58a20aecadc00077dac6f2a04349c4001e8443bb"
    );
}
