//! Individual signing v2 packages at the edges of the manifest contract:
//! manifest bytes outside RFC 8785 or the schema, a second signer, a
//! package stripped of its manifest entries and a package that omits an
//! entry the manifest lists. Manifest defects stay in the manifest
//! component and never read as context problems.

use evidence_verify::report::ComponentKind::{Context, Manifest};
use evidence_verify::report::{Profile, Report};
use evidence_verify::{validate, Outcome};

use crate::individual_v2::complete;
use crate::support::{assert_component, component, fixture, policy, Package};

fn manifest_text() -> String {
    String::from_utf8(fixture!("manifiesto.json").to_vec()).expect("ASCII manifest")
}

/// Validate `package` with `bytes` as its manifest; the overall result is
/// failed and the manifest component fails with `code` while the context
/// carries no manifest code.
fn manifest_fails_with(bytes: &[u8], code: &str) -> Report {
    let mut package = complete();
    package.set("manifiesto.json", bytes);
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.overall, Outcome::Failed, "{code}: {report:#?}");
    assert_component(&report, Manifest, Outcome::Failed, code);
    assert!(
        component(&report, Context)
            .findings
            .iter()
            .all(|finding| !finding.code.starts_with("manifest.")
                && !finding.code.starts_with("json.")),
        "{code}: {report:#?}"
    );
    report
}

#[test]
fn manifest_bytes_outside_the_canonical_form_fail_the_manifest() {
    let text = manifest_text();
    let replace = |from: &str, to: &str| {
        let changed = text.replacen(from, to, 1);
        assert_ne!(changed, text, "{from}");
        changed.into_bytes()
    };
    let with_bom = [&b"\xef\xbb\xbf"[..], text.as_bytes()].concat();
    let with_newline = [text.as_bytes(), b"\n"].concat();
    let cases: Vec<(Vec<u8>, &str)> = vec![
        (with_bom, "json.unexpected_byte"),
        (with_newline, "json.not_canonical"),
        (replace("\"size\":69", "\"size\":69.0"), "json.not_integer"),
        (replace("\"size\":69", "\"size\":null"), "json.literal"),
        (replace("\"size\":69", "\"size\":true"), "json.literal"),
        (
            replace("\"size\":69", "\"size\":9007199254740992"),
            "json.integer_out_of_range",
        ),
        (
            replace("\"size\":69", "\"size\":-1"),
            "json.integer_out_of_range",
        ),
        (
            replace(
                "\"document_version\":1",
                "\"document_version\":1,\"document_version\":1",
            ),
            "json.duplicate_key",
        ),
        (
            replace("\"signer-1\"", "\"\\u0073igner-1\""),
            "json.not_canonical",
        ),
        (
            replace("\"acta.pdf\"", "\"acta\\/pdf\""),
            "json.not_canonical",
        ),
        (
            replace("{\"algorithms\"", "{ \"algorithms\""),
            "json.not_canonical",
        ),
    ];
    for (bytes, code) in cases {
        manifest_fails_with(&bytes, code);
    }
}

#[test]
fn members_outside_the_schema_fail_the_manifest() {
    let text = manifest_text();
    for (from, to) in [
        ("{\"algorithms\"", "{\"aa\":\"x\",\"algorithms\""),
        ("\"size\":69}", "\"size\":69,\"zz\":\"x\"}"),
        ("\"id\":\"signer-1\"}", "\"id\":\"signer-1\",\"zz\":1}"),
        ("\"profile_version\":1", "\"profile_version\":2"),
        ("\"document_version\":1", "\"document_version\":0"),
        ("\"digest\":\"sha-256\"", "\"digest\":\"sha-512\""),
        ("\"role\":\"document\"", "\"role\":\"documento\""),
    ] {
        let changed = text.replacen(from, to, 1);
        assert_ne!(changed, text, "{from}");
        manifest_fails_with(changed.as_bytes(), "manifest.schema");
    }
    let signers = text.find("\"signers\"").expect("signers");
    let empty = format!("{}\"signers\":[]}}", &text[..signers]);
    manifest_fails_with(empty.as_bytes(), "manifest.schema");
}

#[test]
fn a_second_signer_is_not_supported_and_the_token_still_fails() {
    let text = manifest_text();
    let start = text.find("\"signers\":[").expect("signers") + "\"signers\":[".len();
    let first = &text[start..text.len() - 2];
    let second = first.replace("signer-1", "signer-2");
    let bytes = format!("{}{first},{second}]}}", &text[..start]);
    let report = manifest_fails_with(bytes.as_bytes(), "token.imprint_mismatch");
    assert!(
        component(&report, Manifest)
            .findings
            .iter()
            .any(|finding| finding.code == "manifest.signers_not_supported"
                && finding.outcome == Outcome::NotSupported),
        "{report:#?}"
    );
}

#[test]
fn a_package_stripped_of_its_manifest_entries_is_individual_signing_v1() {
    let mut package: Package = complete();
    for entry in [
        "manifiesto.json",
        "manifiesto.json.tsr",
        "tsa-crl.pem",
        "recibo-aceptacion.json",
    ] {
        package.remove(entry);
    }
    let report = validate(&package.zip(), &policy());
    assert_eq!(report.profile, Some(Profile::IndividualSigningV1));
    assert_eq!(report.overall, Outcome::Checked, "{report:#?}");
    assert_eq!(component(&report, Manifest).outcome, Outcome::NotApplicable);
    assert_component(
        &report,
        Context,
        Outcome::Checked,
        "context.records_not_time_stamped",
    );
}

#[test]
fn a_package_that_omits_some_of_the_v2_entries_is_not_supported() {
    for entry in [
        "binding-crl.der",
        "tsa-crl.pem",
        "manifiesto.json.tsr",
        "recibo-aceptacion.json",
    ] {
        let mut package = complete();
        package.remove(entry);
        let report = validate(&package.zip(), &policy());
        assert_eq!(report.overall, Outcome::NotSupported, "{entry}");
        assert!(
            report
                .components
                .iter()
                .flat_map(|component| &component.findings)
                .any(|finding| finding.code == "profile.missing_entry"
                    || finding.code == "profile.unknown_entry"),
            "{entry}: {report:#?}"
        );
    }
}

#[test]
fn an_entry_name_chosen_by_the_manifest_reaches_the_report_only_escaped() {
    let text = manifest_text();
    let hostile = "tsa-crl.pem\u{202e}\n\u{e9}\"x";
    let encoded = "tsa-crl.pem\u{202e}\\n\u{e9}\\\"x";
    let changed = text.replacen("\"tsa-crl.pem\",", &format!("\"{encoded}\","), 1);
    assert_ne!(changed, text);
    let report = manifest_fails_with(changed.as_bytes(), "manifest.artifact_unexpected");
    for finding in &component(&report, Manifest).findings {
        assert!(finding.detail.is_ascii(), "{:?}", finding.detail);
        assert!(!finding.detail.contains(hostile), "{:?}", finding.detail);
    }
    assert_component(&report, Manifest, Outcome::Failed, "manifest.role_mismatch");
}
