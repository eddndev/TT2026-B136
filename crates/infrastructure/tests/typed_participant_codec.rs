use application::ApplicationError;
use infrastructure::typed_participant_codec;
use serde_json::{json, Value};

#[path = "../../domain/tests/typed_participant_vectors_support/mod.rs"]
mod vector_support;

fn vectors() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../domain/tests/fixtures/typed_participant_vectors.json"
    ))
    .unwrap()
}
fn vector(name: &str) -> Value {
    vectors().into_iter().find(|v| v["name"] == name).unwrap()
}
fn decode(canonical: &[u8], input: &Value, is_subject: bool) -> Result<Vec<u8>, ApplicationError> {
    if is_subject {
        typed_participant_codec::subject(canonical, input).map(|v| v.canonical_bytes())
    } else {
        typed_participant_codec::participant(canonical, input).map(|v| v.canonical_bytes())
    }
}
fn rejects(canonical: &[u8], input: &Value, is_subject: bool) {
    let result = decode(canonical, input, is_subject);
    assert!(matches!(
        result,
        Err(ApplicationError::InvalidConfiguration(_))
    ));
    assert_eq!(
        result.unwrap_err().to_string(),
        "invalid application configuration: stored typed participant values are inconsistent"
    );
}

#[test]
fn all_fifteen_subject_and_participant_vectors_reconstruct_the_exact_canonical_bytes() {
    let mut checked = 0;
    for v in vectors() {
        let canonical = bytes(&v);
        if canonical.starts_with(b"PCRED1") {
            continue;
        }
        assert_eq!(
            decode(&canonical, &v["input"], canonical.starts_with(b"SUBJ1")).unwrap(),
            canonical,
            "{}",
            v["name"]
        );
        checked += 1;
    }
    assert_eq!(checked, 15);
}

#[test]
fn every_vector_rejects_damaged_truncated_and_extended_canonical_bytes() {
    for v in vectors() {
        let canonical = bytes(&v);
        if canonical.starts_with(b"PCRED1") {
            continue;
        }
        let is_subject = canonical.starts_with(b"SUBJ1");
        let mut damaged = canonical.clone();
        damaged[0] ^= 1;
        rejects(&damaged, &v["input"], is_subject);
        rejects(&canonical[..canonical.len() - 1], &v["input"], is_subject);
        let mut extended = canonical.clone();
        extended.push(0);
        rejects(&extended, &v["input"], is_subject);
    }
}

#[test]
fn every_projection_object_requires_its_exact_field_inventory() {
    for v in vectors() {
        let canonical = bytes(&v);
        if canonical.starts_with(b"PCRED1") {
            continue;
        }
        let is_subject = canonical.starts_with(b"SUBJ1");
        let mut paths = vec![];
        object_paths(&v["input"], "", &mut paths);
        for path in paths {
            let mut extra = v["input"].clone();
            extra
                .pointer_mut(&path)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert(
                    "unexpected_private_value".into(),
                    json!("do not expose this"),
                );
            rejects(&canonical, &extra, is_subject);
            let keys: Vec<_> = v["input"]
                .pointer(&path)
                .unwrap()
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect();
            for key in keys {
                let mut missing = v["input"].clone();
                missing
                    .pointer_mut(&path)
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .remove(&key);
                rejects(&canonical, &missing, is_subject);
            }
        }
    }
}

#[test]
fn constructors_cannot_silently_trim_identifiers_text_or_empty_optional_values() {
    for (name, path, value) in [
        ("natural_person", "/name/known", json!(" Ana ")),
        ("natural_person", "/curp/unknown", json!(" Not provided")),
        (
            "natural_person",
            "/identity_support/locator",
            json!("page 1 "),
        ),
        (
            "role_defense_counsel",
            "/profile/license/number",
            json!(" 001234"),
        ),
        (
            "role_defense_counsel",
            "/profile/license/issuer",
            json!("Professional Registry "),
        ),
        ("role_defendant", "/organization", json!("")),
        ("role_defendant", "/legal_status", json!("Declared\t")),
        (
            "role_defendant",
            "/subject/id",
            json!("11111111111141118111111111111111"),
        ),
        (
            "role_defendant",
            "/subject/digest",
            json!("F547AC7A3E64A9D2FE550B32E4B8797A74586A95409289ADAC1DFEDA1EC58EDC"),
        ),
    ] {
        let v = vector(name);
        let canonical = bytes(&v);
        let mut input = v["input"].clone();
        *input.pointer_mut(path).unwrap() = value;
        rejects(&canonical, &input, canonical.starts_with(b"SUBJ1"));
    }
}

#[test]
fn unknown_tags_invalid_integers_and_scalar_types_fail_without_panicking() {
    for (name, path, values) in [
        (
            "natural_person",
            "/kind",
            vec![json!("other"), json!(0), Value::Null],
        ),
        (
            "natural_person",
            "/identity_support/version",
            vec![
                json!(0),
                json!(4294967296_u64),
                json!(1.0),
                json!(-1),
                json!("2"),
            ],
        ),
        (
            "role_defendant",
            "/directory_status",
            vec![json!("closed"), Value::Null],
        ),
        (
            "role_defendant",
            "/subject/revision",
            vec![json!(0), json!(u64::MAX), json!(2.5)],
        ),
        (
            "role_defendant",
            "/profile/custody/known",
            vec![json!("missing"), json!(true)],
        ),
        (
            "role_defense_counsel",
            "/profile/mode",
            vec![json!("unknown")],
        ),
        (
            "role_trial_court",
            "/profile/composition",
            vec![json!("unknown")],
        ),
        (
            "role_victim",
            "/profile/contact",
            vec![json!({"unknown":"unknown","documented":{}}), Value::Null],
        ),
        (
            "role_defendant",
            "/organization",
            vec![json!(true), json!({}), json!([])],
        ),
    ] {
        let v = vector(name);
        let canonical = bytes(&v);
        for value in values {
            let mut input = v["input"].clone();
            *input.pointer_mut(path).unwrap() = value;
            rejects(&canonical, &input, canonical.starts_with(b"SUBJ1"));
        }
    }
}

#[test]
fn remaining_declared_variants_and_archived_values_preserve_domain_encoding() {
    let support = vector("natural_person")["input"]["identity_support"].clone();
    let license = json!({"number":"007","issuer":"Registry"});
    for (name, path, value) in [
        (
            "natural_person",
            "/curp",
            json!({"known":"ABCD001122HDFRRN09"}),
        ),
        (
            "institutional_body",
            "/institutional_identifier",
            json!({"unknown":"Awaiting official record"}),
        ),
        (
            "role_defendant",
            "/profile/custody",
            json!({"unknown":"Not recorded"}),
        ),
        (
            "role_defendant",
            "/profile/custody",
            json!({"known":"detained"}),
        ),
        ("role_defendant", "/directory_status", json!("archived")),
        ("role_defendant", "/organization", json!("Organization")),
        ("role_defendant", "/legal_status", Value::Null),
        (
            "role_victim",
            "/profile/contact",
            json!({"unknown":"Not recorded"}),
        ),
        (
            "role_victim",
            "/profile/contact",
            json!({"none":"No contact requested"}),
        ),
        (
            "role_victim",
            "/profile/protection",
            json!({"unknown":"Not recorded"}),
        ),
        (
            "role_victim",
            "/profile/protection",
            json!({"documented":support}),
        ),
        ("role_defense_counsel", "/profile/mode", json!("public")),
        ("role_trial_court", "/profile/composition", json!("single")),
        (
            "role_victim_counsel",
            "/profile/license",
            json!({"known":license}),
        ),
        (
            "role_prosecutor",
            "/profile/license",
            json!({"unknown":"Not recorded"}),
        ),
    ] {
        let mut input = vector(name)["input"].clone();
        *input.pointer_mut(path).unwrap() = value;
        let is_subject = input.get("kind").is_some();
        let canonical = if is_subject {
            vector_support::subject(&input).canonical_bytes()
        } else {
            vector_support::participant(&input).canonical_bytes()
        };
        assert_eq!(decode(&canonical, &input, is_subject).unwrap(), canonical);
        if name == "natural_person" {
            input["curp"]["known"] = json!("abcd001122hdfrrn09");
            rejects(&canonical, &input, true);
        }
    }
}

fn object_paths(value: &Value, path: &str, result: &mut Vec<String>) {
    if let Some(fields) = value.as_object() {
        result.push(path.to_owned());
        for (key, value) in fields {
            object_paths(value, &format!("{path}/{key}"), result);
        }
    }
}

fn bytes(v: &Value) -> Vec<u8> {
    v["hex"]
        .as_str()
        .unwrap()
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
