use application::{hearings::HearingError, ApplicationError};
use infrastructure::hearing_codec;
use serde_json::{json, Value};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

fn vectors() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../domain/tests/fixtures/hearing_vectors.json"
    ))
    .unwrap()
}
fn bytes(vector: &Value) -> Vec<u8> {
    vector["hex"]
        .as_str()
        .unwrap()
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn projection(vector: &Value) -> Value {
    let mut value = vector["input"].clone();
    let at = OffsetDateTime::parse(value["time"].as_str().unwrap(), &Rfc3339).unwrap();
    value["time"] =
        json!({"seconds":at.unix_timestamp(),"offset_seconds":at.offset().whole_seconds()});
    value["participants"]
        .as_array_mut()
        .unwrap()
        .sort_by_key(|v| v["id"].as_str().unwrap().to_owned());
    value
}
fn rejects(bytes: &[u8], projection: &Value) {
    assert!(matches!(
        hearing_codec::values(bytes, projection),
        Err(ApplicationError::Hearing(HearingError::StoredInconsistent(
            _
        )))
    ));
}

#[test]
fn reconstructs_every_independent_hearing_vector_without_changing_exact_bytes() {
    for vector in vectors() {
        let bytes = bytes(&vector);
        let actual = hearing_codec::values(&bytes, &projection(&vector)).unwrap();
        assert_eq!(actual.canonical_bytes(), bytes, "{}", vector["name"]);
    }
}

#[test]
fn rejects_damaged_canonical_bytes_even_when_the_projection_is_valid() {
    for vector in vectors() {
        let bytes = bytes(&vector);
        let projected = projection(&vector);
        rejects(&bytes[..bytes.len() - 1], &projected);
        let mut trailing = bytes.clone();
        trailing.push(0);
        rejects(&trailing, &projected);
        let mut damaged = bytes;
        damaged[0] ^= 1;
        rejects(&damaged, &projected);
    }
}

#[test]
fn rejects_normalizable_text_unsorted_references_and_noncanonical_identifiers() {
    let vector = vectors().remove(3);
    let bytes = bytes(&vector);
    let value = projection(&vector);
    for (path, changed) in [
        ("/venue", json!(" Court A ")),
        ("/note", json!("\u{00e1}\r\nb")),
        (
            "/conviction_basis/statement",
            json!(" Declared\nConviction \u{00e1}"),
        ),
        (
            "/conviction_basis/document_id",
            json!("00112233-4455-6677-8899-AABBCCDDEEFF"),
        ),
        (
            "/conviction_basis/digest",
            json!(value["conviction_basis"]["digest"]
                .as_str()
                .unwrap()
                .to_uppercase()),
        ),
        ("/time/seconds", json!(1767175800.0)),
        (
            "/participants/0/id",
            json!("00000000000000000000000000000001"),
        ),
    ] {
        let mut changed_value = value.clone();
        *changed_value.pointer_mut(path).unwrap() = changed;
        rejects(&bytes, &changed_value);
    }
    let mut reversed = value.clone();
    reversed["participants"].as_array_mut().unwrap().reverse();
    rejects(&bytes, &reversed);
}

#[test]
fn every_projected_object_requires_exact_fields() {
    let vector = vectors().remove(3);
    let bytes = bytes(&vector);
    let value = projection(&vector);
    for path in ["", "/time", "/participants/0", "/conviction_basis"] {
        let mut extra = value.clone();
        extra
            .pointer_mut(path)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), json!("extra"));
        rejects(&bytes, &extra);
        for key in value.pointer(path).unwrap().as_object().unwrap().keys() {
            let mut missing = value.clone();
            missing
                .pointer_mut(path)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(key);
            rejects(&bytes, &missing);
        }
    }
}
