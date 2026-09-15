use super::{
    accepted,
    fixtures::{self, document, WORD},
    rejected,
};
use crate::document_formats::docx::{validate_docx, DocxBudget};
use application::ApplicationError;

#[test]
fn malformed_xml_names_entities_namespaces_and_trailing_content_are_rejected() {
    for bad in [
        "<w:1bad/>",
        "<p:item/>",
        "<w:p a='1' a='2'/>",
        "<w:p xmlns:a='urn:x' xmlns:b='urn:x' a:v='1' b:v='2'/>",
        "<w:p>&unknown;</w:p>",
        "<w:p>&#0;</w:p>",
        "<w:p>&#xFFFF;</w:p>",
        "<w:p>&dangling</w:p>",
        "<w:p a='&#1;'/>",
        "<w:p a='<bad'/>",
        "<w:p xmlns:xml='urn:wrong'/>",
        "<w:p xmlns:xmlns='urn:wrong'/>",
        "<w:p xmlns:a=''/>",
        "<xmlns:p/>",
        "<w:p xmlns:a='urn:bad space'/>",
        "<w:p>bad\x01</w:p>",
        "<w:p></w:q>",
    ] {
        rejected(&fixtures::normal(&document(WORD, bad), &[]));
    }
    for bad in [
        "<!DOCTYPE a [<!ENTITY evil 'x'>]><w:document xmlns:w='x'/>",
        "<a/><b/>",
        "<a/>text",
        "<?xml version='1.1'?><a/>",
        "<?xml version='1.0' encoding='UTF-16'?><a/>",
        "<a/><?xml version='1.0'?>",
    ] {
        rejected(&fixtures::normal(bad, &[]));
    }
}

#[test]
fn predefined_entities_numeric_characters_and_utf8_bom_are_accepted() {
    let xml = format!(
        "\u{feff}{}",
        document(
            WORD,
            "<w:p a='&amp;&#10;'>&lt;&gt;&amp;&apos;&quot;&#x1F600;</w:p>"
        )
    );
    accepted(&fixtures::normal(&xml, &[]));
}

#[test]
fn nesting_and_event_counts_use_explicit_limits() {
    let deeply_nested = format!("{}{}", "<w:p>".repeat(128), "</w:p>".repeat(128));
    let xml = document(WORD, &deeply_nested);
    assert!(matches!(
        validate_docx(&fixtures::normal(&xml, &[]), &mut DocxBudget::standard()),
        Err(ApplicationError::StageSupportValidationLimit)
    ));
    let xml = document(WORD, &"<w:p/>".repeat(1_000_001));
    assert!(matches!(
        validate_docx(&fixtures::normal(&xml, &[]), &mut DocxBudget::standard()),
        Err(ApplicationError::StageSupportValidationLimit)
    ));
}

#[test]
fn the_xml_event_budget_is_shared_across_successive_docx_files() {
    let xml = document(WORD, &"<w:p/>".repeat(600_000));
    let bytes = fixtures::normal(&xml, &[]);
    let mut budget = DocxBudget::standard();
    validate_docx(&bytes, &mut budget).unwrap();
    assert!(matches!(
        validate_docx(&bytes, &mut budget),
        Err(ApplicationError::StageSupportValidationLimit)
    ));
}

#[test]
fn the_maximum_depth_is_accepted_without_an_extra_level() {
    let nested = format!("{}{}", "<w:p>".repeat(126), "</w:p>".repeat(126));
    accepted(&fixtures::normal(&document(WORD, &nested), &[]));
}
