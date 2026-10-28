//! The JSON contract between the Web Worker and the evidence validator:
//! policy parsing, report encoding and the end-to-end entry point. The
//! contract is `docs/adr/0085-in-browser-evidence-validation.md`.

use evidence_verify::policy::{Purpose, Purposes, DEFAULT_ADMITTED_POLICY};
use evidence_verify::report::{
    Component, ComponentKind, Instant, InstantSource, Profile, Report, TrustNote,
};
use evidence_verify::{Finding, Outcome};
use evidence_verify_wasm::policy_json::{parse_policy, PolicyError};
use evidence_verify_wasm::report_json::report_value;
use evidence_verify_wasm::{validate_json, ENGINE_VERSION};
use serde_json::{json, Value};

const FINGERPRINT: &str = "98f934f7152cd0b12ef1bf9b424a62d425a02496b531e218d2cde45a4130a4b6";

fn policy(pinned: Value, served: Value) -> String {
    json!({"pinned": pinned, "served": served, "current_instant": 1_800_000_000i64}).to_string()
}

fn pinned(fingerprint: &str, purposes: Value) -> Value {
    json!([{"fingerprint": fingerprint, "purposes": purposes, "label": "Qadra"}])
}

#[test]
fn every_purpose_parses_with_the_fingerprint_and_label() {
    let text = policy(
        pinned(
            FINGERPRINT,
            json!(["server-seal", "individual-signing", "time-stamping"]),
        ),
        Value::Null,
    );
    let parsed = parse_policy(&text).expect("valid policy");
    assert_eq!(parsed.pinned.len(), 1);
    let root = &parsed.pinned[0];
    assert_eq!(root.purposes, Purposes::ALL);
    assert_eq!(root.label, "Qadra");
    assert_eq!(root.sha256[0], 0x98);
    assert_eq!(root.sha256[31], 0xb6);
    assert_eq!(parsed.current_instant, 1_800_000_000);
    for purpose in Purpose::ALL {
        let text = policy(pinned(FINGERPRINT, json!([purpose.name()])), Value::Null);
        let parsed = parse_policy(&text).expect("single purpose");
        assert!(parsed.pinned[0].purposes.allows(purpose));
        let others = Purpose::ALL.into_iter().filter(|other| *other != purpose);
        for other in others {
            assert!(!parsed.pinned[0].purposes.allows(other));
        }
    }
}

#[test]
fn malformed_fingerprints_are_rejected() {
    let upper = FINGERPRINT.to_uppercase();
    let short = &FINGERPRINT[..63];
    let not_hex = format!("{}g", &FINGERPRINT[..63]);
    for fingerprint in [upper.as_str(), short, not_hex.as_str(), ""] {
        let text = policy(pinned(fingerprint, json!(["server-seal"])), Value::Null);
        assert_eq!(
            parse_policy(&text),
            Err(PolicyError::Fingerprint),
            "{fingerprint}"
        );
    }
}

#[test]
fn unknown_or_missing_purposes_are_rejected() {
    for purposes in [json!(["server"]), json!([]), json!(["Server-Seal"])] {
        let text = policy(pinned(FINGERPRINT, purposes.clone()), Value::Null);
        assert_eq!(parse_policy(&text), Err(PolicyError::Purpose), "{purposes}");
    }
    let served =
        json!({"roots": [{"der_base64": "MAA=", "purposes": ["other"]}], "crls_base64": []});
    let text = policy(json!([]), served);
    assert_eq!(parse_policy(&text), Err(PolicyError::Purpose));
}

#[test]
fn bad_base64_is_rejected_in_roots_and_crls() {
    let root = json!({"roots": [{"der_base64": "not base64!", "purposes": ["server-seal"]}], "crls_base64": []});
    assert_eq!(
        parse_policy(&policy(json!([]), root)),
        Err(PolicyError::Base64)
    );
    let crl = json!({"roots": [], "crls_base64": ["%%%"]});
    assert_eq!(
        parse_policy(&policy(json!([]), crl)),
        Err(PolicyError::Base64)
    );
}

#[test]
fn malformed_documents_are_rejected() {
    let cases = [
        "not json".to_owned(),
        "[]".to_owned(),
        json!({"pinned": [], "served": null}).to_string(),
        json!({"pinned": [], "served": null, "current_instant": "now"}).to_string(),
        json!({"pinned": [], "served": null, "current_instant": 1.5}).to_string(),
        json!({"pinned": {}, "served": null, "current_instant": 1}).to_string(),
        json!({"pinned": [{"fingerprint": FINGERPRINT, "purposes": ["server-seal"]}],
            "served": null, "current_instant": 1})
        .to_string(),
        json!({"pinned": [], "served": {"roots": []}, "current_instant": 1}).to_string(),
        json!({"pinned": [], "served": null, "admitted_policies": [1], "current_instant": 1})
            .to_string(),
    ];
    for text in cases {
        assert_eq!(parse_policy(&text), Err(PolicyError::Malformed), "{text}");
    }
}

#[test]
fn error_codes_are_stable() {
    assert_eq!(PolicyError::Malformed.code(), "policy.malformed");
    assert_eq!(PolicyError::Fingerprint.code(), "policy.fingerprint");
    assert_eq!(PolicyError::Purpose.code(), "policy.purpose");
    assert_eq!(PolicyError::Base64.code(), "policy.base64");
}

#[test]
fn omitted_admitted_policies_take_the_default() {
    let parsed = parse_policy(&policy(json!([]), Value::Null)).expect("valid");
    assert_eq!(parsed.admitted_policies, vec![DEFAULT_ADMITTED_POLICY]);
    let text = json!({"pinned": [], "served": null, "admitted_policies": ["1.2.3"],
        "current_instant": -5})
    .to_string();
    let parsed = parse_policy(&text).expect("valid");
    assert_eq!(parsed.admitted_policies, vec!["1.2.3"]);
    assert_eq!(parsed.current_instant, -5);
}

#[test]
fn null_served_trust_gives_none_and_an_object_is_decoded() {
    let parsed = parse_policy(&policy(json!([]), Value::Null)).expect("valid");
    assert_eq!(parsed.served, None);
    let served = json!({
        "roots": [{"der_base64": "MAA=", "purposes": ["server-seal", "time-stamping"]}],
        "crls_base64": ["AQID", "BAU="]
    });
    let parsed = parse_policy(&policy(json!([]), served)).expect("valid");
    let served = parsed.served.expect("served trust");
    assert_eq!(served.roots.len(), 1);
    assert_eq!(served.roots[0].der, vec![0x30, 0x00]);
    assert!(served.roots[0].purposes.server_seal);
    assert!(served.roots[0].purposes.time_stamping);
    assert!(!served.roots[0].purposes.individual_signing);
    assert_eq!(served.crls, vec![vec![1, 2, 3], vec![4, 5]]);
}

fn finding(outcome: Outcome) -> Finding {
    Finding::new(outcome, "test.code", "detail")
}

fn hand_built(profile: Option<Profile>) -> Report {
    let outcomes = [
        Outcome::Checked,
        Outcome::Failed,
        Outcome::Indeterminate,
        Outcome::NotSupported,
    ];
    let mut components: Vec<Component> = ComponentKind::ALL
        .iter()
        .zip(outcomes.iter().cycle())
        .map(|(kind, outcome)| Component::new(*kind, vec![finding(*outcome)]))
        .collect();
    components[7] = Component::not_applicable(ComponentKind::Context);
    let mut report = Report::from_components(profile, components);
    report.instants = vec![
        Instant {
            label: "token".into(),
            unix: 1,
            source: InstantSource::Token,
        },
        Instant {
            label: "record".into(),
            unix: 2,
            source: InstantSource::ApplicationRecord,
        },
        Instant {
            label: "now".into(),
            unix: 3,
            source: InstantSource::Caller,
        },
    ];
    report.digests = vec![("document.bin".into(), "ab".repeat(32))];
    report.trust = Purpose::ALL
        .into_iter()
        .map(Some)
        .chain([None])
        .map(|purpose| TrustNote {
            code: "trust.note",
            purpose,
            fingerprint: String::new(),
            detail: "d".into(),
        })
        .collect();
    report.current_status = vec![Finding::new(Outcome::NotApplicable, "current.x", "")];
    report
}

fn strings(value: &Value, pointer: &str, key: &str) -> Vec<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .expect("array")
        .iter()
        .map(|item| item[key].as_str().map(str::to_owned).unwrap_or_default())
        .collect()
}

#[test]
fn the_encoder_emits_every_enum_spelling() {
    let value = report_value(&hand_built(Some(Profile::ServerSealV1)));
    assert_eq!(value["profile"], "server-seal-v1");
    assert_eq!(value["overall"], "failed");
    assert_eq!(
        strings(&value, "/components", "kind"),
        [
            "package",
            "document-token",
            "document-signature",
            "binding-consent",
            "certificates",
            "revocation",
            "manifest",
            "context"
        ]
    );
    assert_eq!(
        strings(&value, "/components", "outcome"),
        [
            "checked",
            "failed",
            "indeterminate",
            "not-supported",
            "checked",
            "failed",
            "indeterminate",
            "not-applicable"
        ]
    );
    assert_eq!(
        value["components"][1]["findings"],
        json!([{"outcome": "failed", "code": "test.code", "detail": "detail"}])
    );
    assert_eq!(value["components"][7]["findings"], json!([]));
    assert_eq!(
        strings(&value, "/instants", "source"),
        ["token", "application-record", "caller"]
    );
    assert_eq!(
        value["instants"][0],
        json!({"label": "token", "unix": 1, "source": "token"})
    );
    assert_eq!(
        value["digests"],
        json!([{"name": "document.bin", "sha256": "ab".repeat(32)}])
    );
    let purposes: Vec<Value> = value["trust"]
        .as_array()
        .expect("trust")
        .iter()
        .map(|note| note["purpose"].clone())
        .collect();
    assert_eq!(
        purposes,
        [
            json!("server-seal"),
            json!("individual-signing"),
            json!("time-stamping"),
            Value::Null
        ]
    );
    assert_eq!(
        value["trust"][0],
        json!({"code": "trust.note", "purpose": "server-seal", "fingerprint": "", "detail": "d"})
    );
    assert_eq!(
        value["current_status"],
        json!([{"outcome": "not-applicable", "code": "current.x", "detail": ""}])
    );
    for (profile, name) in [
        (
            Some(Profile::IndividualSigningV1),
            json!("individual-signing-v1"),
        ),
        (
            Some(Profile::IndividualSigningV2),
            json!("individual-signing-v2"),
        ),
        (None, Value::Null),
    ] {
        assert_eq!(report_value(&hand_built(profile))["profile"], name);
    }
    let mut report = hand_built(None);
    for overall in [
        Outcome::Checked,
        Outcome::Indeterminate,
        Outcome::NotSupported,
    ] {
        report.overall = overall;
        let expected = match overall {
            Outcome::Checked => "checked",
            Outcome::Indeterminate => "indeterminate",
            _ => "not-supported",
        };
        assert_eq!(report_value(&report)["overall"], expected);
    }
}

#[test]
fn a_file_that_is_not_a_zip_is_not_supported_with_a_package_reason() {
    let text = validate_json(b"not a zip", &policy(json!([]), Value::Null)).expect("report");
    let value: Value = serde_json::from_str(&text).expect("json");
    assert_eq!(
        value["engine"],
        json!({"name": "evidence-verify", "version": ENGINE_VERSION})
    );
    assert_eq!(ENGINE_VERSION, env!("CARGO_PKG_VERSION"));
    assert_eq!(value["overall"], "not-supported");
    assert_eq!(value["profile"], Value::Null);
    let package = &value["components"][0];
    assert_eq!(package["kind"], "package");
    assert_eq!(package["outcome"], "not-supported");
    let code = package["findings"][0]["code"].as_str().expect("code");
    assert!(code.starts_with("zip."), "{code}");
    for key in ["instants", "digests", "trust", "current_status"] {
        assert!(value[key].is_array(), "{key}");
    }
}

#[test]
fn a_bad_policy_is_an_error_before_validation() {
    assert_eq!(
        validate_json(b"not a zip", "{}"),
        Err(PolicyError::Malformed)
    );
}
