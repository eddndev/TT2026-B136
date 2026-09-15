use domain::document_metadata::{DocumentMetadata, MetadataRevision};
use domain::identity::{Permission, Role};
use domain::DomainError;

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).into()).collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn canonical_vectors_preserve_lengths_optionality_and_utf8() {
    for (kind, class, tags, expected) in [
        (None, None, vec![], "444d45544131000000000000"),
        (
            Some("Escrito"),
            None,
            strings(&["a", "z"]),
            "444d4554413101000000074573637269746f00000000020000000161000000017a",
        ),
        (
            Some("Tipo"),
            Some("Civil"),
            strings(&["a,b", "\u{e1}"]),
            "444d4554413101000000045469706f0100000005436976696c0000000200000003612c6200000002c3a1",
        ),
    ] {
        assert_eq!(
            hex(&DocumentMetadata::new(kind, class, &tags)
                .unwrap()
                .canonical_bytes()),
            expected
        );
    }
}

#[test]
fn normalization_trims_deduplicates_and_sorts_without_casefold_or_nfc() {
    let metadata = DocumentMetadata::new(
        Some("  Escrito  "),
        Some("\u{2003} "),
        &strings(&[" z ", "a", "a ", "A", "\u{e1}", "a\u{301}", "a,b", "a  b"]),
    )
    .unwrap();
    assert_eq!(metadata.document_type(), Some("Escrito"));
    assert_eq!(metadata.classification(), None);
    assert_eq!(
        metadata.tags(),
        strings(&["A", "a", "a  b", "a,b", "a\u{301}", "z", "\u{e1}"])
    );
    assert_eq!(
        DocumentMetadata::empty(),
        DocumentMetadata::new(Some(" "), None, &[]).unwrap()
    );
}

#[test]
fn every_original_control_is_rejected_before_trimming() {
    for control in ['\0', '\n', '\r', '\t', '\u{7f}', '\u{85}'] {
        let value = format!("{control}text{control}");
        for result in [
            DocumentMetadata::new(Some(&value), None, &[]),
            DocumentMetadata::new(None, Some(&value), &[]),
            DocumentMetadata::new(None, None, std::slice::from_ref(&value)),
        ] {
            assert!(matches!(
                result,
                Err(DomainError::InvalidDocumentMetadata { .. })
            ));
            assert!(!result.unwrap_err().to_string().contains(&value));
        }
    }
}

#[test]
fn scalar_limits_and_raw_tag_count_are_enforced() {
    let eighty = "\u{1f600}".repeat(80);
    let forty = "\u{1f600}".repeat(40);
    let max = DocumentMetadata::new(
        Some(&eighty),
        Some(&eighty),
        &(0..20)
            .map(|i| {
                format!(
                    "{}{}",
                    "\u{1f600}".repeat(39),
                    char::from_u32(0x1f600 + i).unwrap()
                )
            })
            .collect::<Vec<_>>(),
    )
    .unwrap();
    assert_eq!(max.canonical_bytes().len(), 3940);
    assert!(DocumentMetadata::new(Some(&(eighty.clone() + "a")), None, &[]).is_err());
    assert!(DocumentMetadata::new(None, Some(&(eighty + "a")), &[]).is_err());
    assert!(DocumentMetadata::new(None, None, std::slice::from_ref(&forty)).is_ok());
    assert!(DocumentMetadata::new(None, None, &[forty + "a"]).is_err());
    assert!(DocumentMetadata::new(None, None, &vec!["a".into(); 21]).is_err());
    assert!(DocumentMetadata::new(None, None, &strings(&["a", " "])).is_err());
    assert!(DocumentMetadata::new(None, None, &strings(&[""])).is_err());
}

#[test]
fn canonical_structure_distinguishes_fields_and_tag_boundaries() {
    let variants = [
        DocumentMetadata::new(Some("a"), None, &[]).unwrap(),
        DocumentMetadata::new(None, Some("a"), &[]).unwrap(),
        DocumentMetadata::new(None, None, &strings(&["a"])).unwrap(),
        DocumentMetadata::new(None, None, &strings(&["a,b"])).unwrap(),
        DocumentMetadata::new(None, None, &strings(&["a", "b"])).unwrap(),
    ];
    for (i, left) in variants.iter().enumerate() {
        for right in &variants[i + 1..] {
            assert_ne!(left.canonical_bytes(), right.canonical_bytes());
        }
    }
}

#[test]
fn metadata_revision_zero_and_exhaustion_are_explicit() {
    assert_eq!(MetadataRevision::unclassified().get(), 0);
    assert_eq!(
        MetadataRevision::unclassified().next(),
        Some(MetadataRevision::new(1))
    );
    assert!(MetadataRevision::new(1) < MetadataRevision::new(2));
    assert_eq!(MetadataRevision::new(u32::MAX).next(), None);
}

#[test]
fn only_staff_roles_may_classify_documents() {
    for role in [Role::Owner, Role::Litigator, Role::Paralegal] {
        assert!(role.allows(Permission::ClassifyDocument));
    }
    assert!(!Role::Client.allows(Permission::ClassifyDocument));
}
