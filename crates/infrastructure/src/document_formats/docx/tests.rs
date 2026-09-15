mod fixtures;
mod raw_zip;
mod xml_cases;
mod zip_cases;

use super::{validate_docx, DocxBudget};
use application::ApplicationError;
use fixtures::{document, package, Part, WORD};

fn accepted(bytes: &[u8]) {
    validate_docx(bytes, &mut DocxBudget::standard()).unwrap();
}
fn rejected(bytes: &[u8]) {
    assert!(matches!(
        validate_docx(bytes, &mut DocxBudget::standard()),
        Err(ApplicationError::StageSupportFormatRejected)
    ));
}

#[test]
fn minimal_transitional_and_strict_packages_resolve_their_main_relationship() {
    for (namespace, relation, path) in [
        (
            WORD,
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument",
            "word/document.xml",
        ),
        (
            "http://purl.oclc.org/ooxml/wordprocessingml/main",
            "http://purl.oclc.org/ooxml/officeDocument/relationships/officeDocument",
            "alternate/main.xml",
        ),
    ] {
        accepted(&package(
            path,
            &document(namespace, "<w:p/>"),
            relation,
            &[],
        ));
    }
}

#[test]
fn main_part_content_type_root_namespace_and_internal_targets_are_required() {
    for body in [
        "<other/>",
        "<document xmlns='wrong'/>",
        "<w:document xmlns:w='http://schemas.openxmlformats.org/wordprocessingml/2006/main'/>",
    ] {
        rejected(&fixtures::normal(body, &[]));
    }
    let broken = package("missing.xml", &document(WORD, ""), "urn:unknown", &[]);
    rejected(&broken);
}

#[test]
fn macro_embedded_object_and_external_media_relationships_are_rejected() {
    for kind in [
        "vbaProject",
        "oleObject",
        "package",
        "control",
        "attachedTemplate",
        "image",
    ] {
        let xml = fixtures::relationships(&format!("<Relationship Id='r2' Type='http://schemas.openxmlformats.org/officeDocument/2006/relationships/{kind}' Target='https://example.test/file' TargetMode='External'/>"));
        rejected(&fixtures::normal(
            &document(WORD, ""),
            &[Part::text("word/_rels/document.xml.rels", &xml)],
        ));
    }
}

#[test]
fn external_hyperlinks_are_inert_and_internal_targets_must_exist() {
    let good = fixtures::relationships("<Relationship Id='r2' Type='http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink' Target='https://example.test/' TargetMode='External'/>");
    accepted(&fixtures::normal(
        &document(WORD, ""),
        &[Part::text("word/_rels/document.xml.rels", &good)],
    ));
    let missing = fixtures::relationships("<Relationship Id='r2' Type='http://schemas.openxmlformats.org/officeDocument/2006/relationships/image' Target='media/missing.png'/>");
    rejected(&fixtures::normal(
        &document(WORD, ""),
        &[Part::text("word/_rels/document.xml.rels", &missing)],
    ));
}

#[test]
fn all_decompressed_bytes_share_one_budget_across_packages() {
    let media = Part {
        name: "word/media/image.bin".into(),
        bytes: vec![0; 34 * 1024 * 1024],
    };
    let bytes = fixtures::normal(&document(WORD, ""), &[media]);
    let mut budget = DocxBudget::standard();
    validate_docx(&bytes, &mut budget).unwrap();
    assert!(matches!(
        validate_docx(&bytes, &mut budget),
        Err(ApplicationError::StageSupportValidationLimit)
    ));
}

#[test]
fn actual_entry_bytes_must_match_sizes_and_crc_after_complete_drain() {
    let mut bytes = fixtures::normal(&document(WORD, ""), &[]);
    let central = fixtures::signature(&bytes, b"PK\x01\x02");
    bytes[central + 16] ^= 1;
    bytes[14] ^= 1;
    rejected(&bytes);
}

#[test]
fn a_python_docx_producer_package_is_accepted() {
    accepted(include_bytes!("tests/fixtures/producer.docx"));
}

#[test]
fn main_and_body_word_namespaces_cannot_mix_strict_and_transitional() {
    let mixed = format!("<w:document xmlns:w='{WORD}'><s:body xmlns:s='http://purl.oclc.org/ooxml/wordprocessingml/main'/></w:document>");
    rejected(&fixtures::normal(&mixed, &[]));
}
