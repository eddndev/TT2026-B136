mod typed_participant_vectors_support;
use serde_json::Value;
use typed_participant_vectors_support::{participant, subject};

#[test]
fn independent_subject_and_all_role_vectors_match_exact_canonical_bytes() {
    let vectors: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/typed_participant_vectors.json")).unwrap();
    let mut checked = 0;
    for vector in vectors {
        let name = vector["name"].as_str().unwrap();
        if name.starts_with("credential_") {
            continue;
        }
        let bytes = if vector["input"].get("subject").is_some() {
            participant(&vector["input"]).canonical_bytes()
        } else {
            subject(&vector["input"]).canonical_bytes()
        };
        let hex = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        assert_eq!(
            bytes.len(),
            vector["bytes"].as_u64().unwrap() as usize,
            "{name}"
        );
        assert_eq!(hex, vector["hex"].as_str().unwrap(), "{name}");
        checked += 1;
    }
    assert_eq!(checked, 15);
}
