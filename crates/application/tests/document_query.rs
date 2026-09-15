use application::documents::DocumentQuery;
use application::ApplicationError;

#[test]
fn document_queries_bound_page_size_without_truncating_offsets() {
    for limit in [1, 100] {
        let query = DocumentQuery::new(limit, u32::MAX, None, None).unwrap();
        assert_eq!(query.limit(), limit);
        assert_eq!(query.offset(), u32::MAX);
        assert_eq!(query.name(), None);
        assert_eq!(query.sealed(), None);
    }
    for limit in [0, 101, u32::MAX] {
        assert!(matches!(
            DocumentQuery::new(limit, 0, None, None),
            Err(ApplicationError::InvalidInput(_))
        ));
    }
}

#[test]
fn name_search_trims_spaces_and_preserves_literal_characters() {
    let query = DocumentQuery::new(20, 2, Some("  Proof_100%.PDF  "), Some(false)).unwrap();
    assert_eq!(query.name(), Some("Proof_100%.PDF"));
    assert_eq!(query.sealed(), Some(false));
    assert_eq!(
        DocumentQuery::new(20, 0, Some("   "), Some(true))
            .unwrap()
            .name(),
        None
    );
}

#[test]
fn name_search_bounds_characters_and_rejects_controls_even_at_edges() {
    let maximum = "\u{e9}".repeat(200);
    assert!(DocumentQuery::new(20, 0, Some(&maximum), None).is_ok());
    for name in [
        "a".repeat(201),
        "a\nb".into(),
        "\tproof".into(),
        "proof\0".into(),
    ] {
        assert!(matches!(
            DocumentQuery::new(20, 0, Some(&name), None),
            Err(ApplicationError::InvalidInput(_))
        ));
    }
}
