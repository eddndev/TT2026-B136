use super::{profile::Profile, values::SubjectValues};
use serde_json::{json, Value};

fn support() -> Value {
    json!({"document_id":"11111111-1111-4111-8111-111111111111",
        "version":2,"digest":"12".repeat(32),"locator":"Page 2"})
}
fn person() -> Value {
    json!({"kind":"natural_person","name":{"state":"known","value":"Ana"},
        "curp":{"state":"unknown","reason":"Not supplied"},
        "identity_support":support()})
}

#[test]
fn subject_wire_roundtrip_preserves_unknown_identity_and_exact_support() {
    let mut value = person();
    value["name"] = json!({"state":"unidentified","label":"Person A", "reason":"Unknown"});
    let input: SubjectValues = serde_json::from_value(value.clone()).unwrap();
    let domain = input.validate().unwrap();
    assert_eq!(domain.display_name(), "Person A");
    assert_eq!(
        serde_json::to_value(SubjectValues::from(&domain)).unwrap(),
        value
    );
}

#[test]
fn all_eleven_profiles_have_distinct_lossless_wire_shapes() {
    let unknown = json!({"state":"unknown","reason":"Not supplied"});
    let license = json!({"number":"0000123","issuer":"Declared issuer"});
    let profiles = [
        json!({"kind":"defendant","custody":{"state":"known","value":"detained"}}),
        json!({"kind":"victim","contact":{"state":"documented","support":support()},
            "protection":{"state":"none_declared","reason":"None declared"}}),
        json!({"kind":"defense_counsel","license":license,"mode":"private"}),
        json!({"kind":"prosecutor","office_identifier":unknown,"unit":unknown,"license":unknown}),
        json!({"kind":"victim_counsel","institution":"Institute","license":{"state":"known","value":license}}),
        json!({"kind":"control_judge","court":"Court 4"}),
        json!({"kind":"trial_court","judicial_district":"District 1","composition":"collegiate"}),
        json!({"kind":"expert","specialty":unknown,"license":unknown}),
        json!({"kind":"police","agency":unknown,"unit":unknown}),
        json!({"kind":"precautionary_supervisor","authority":unknown,"unit":unknown}),
        json!({"kind":"other","label":"Interpreter","description":unknown}),
    ];
    for (tag, value) in profiles.into_iter().enumerate() {
        let input: Profile = serde_json::from_value(value.clone()).unwrap();
        let domain = input.validate().unwrap();
        assert_eq!(domain.kind().tag() as usize, tag);
        assert_eq!(serde_json::to_value(Profile::from(&domain)).unwrap(), value);
    }
}

#[test]
fn variants_reject_foreign_unknown_and_duplicate_fields() {
    for invalid in [
        r#"{"kind":"control_judge","court":"A","court":"B"}"#,
        r#"{"kind":"control_judge","kind":"expert","court":"A"}"#,
        r#"{"kind":"control_judge","court":"A","license":null}"#,
        r#"{"kind":"defendant","custody":{"state":"unknown","reason":"Missing","value":"detained"}}"#,
    ] {
        assert!(
            serde_json::from_str::<Profile>(invalid).is_err(),
            "{invalid}"
        );
    }
    let mut invalid = person();
    invalid["institutional_identifier"] = json!({"state":"unknown","reason":"Missing"});
    assert!(serde_json::from_value::<SubjectValues>(invalid).is_err());
}

#[test]
fn values_reject_controls_bad_evidence_and_malformed_declared_identifiers() {
    for (key, value) in [
        ("name", json!({"state":"known","value":"\tAna"})),
        ("curp", json!({"state":"known","value":"ABC"})),
        ("curp", json!({"state":"unknown","reason":" "})),
        (
            "identity_support",
            json!({"document_id":"11111111-1111-4111-8111-111111111111",
            "version":0,"digest":"12".repeat(32),"locator":"Page 2"}),
        ),
        (
            "identity_support",
            json!({"document_id":"11111111-1111-4111-8111-111111111111",
            "version":2,"digest":"AB".repeat(32),"locator":"Page 2"}),
        ),
    ] {
        let mut input = person();
        input[key] = value;
        assert!(serde_json::from_value::<SubjectValues>(input)
            .unwrap()
            .validate()
            .is_err());
    }
}
