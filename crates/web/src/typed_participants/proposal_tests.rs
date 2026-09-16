use super::proposal::{Preparation, ReviewRequest, Submission};
use serde_json::{json, Value};

fn reference() -> Value {
    json!({"id":"22222222-2222-4222-8222-222222222222","revision":1,"values_digest":"22".repeat(32)})
}
fn role() -> Value {
    json!({"profile":{"kind":"control_judge","court":"Court"},
        "role_support":{"document_id":"11111111-1111-4111-8111-111111111111",
            "version":1,"digest":"11".repeat(32),"locator":"Page 1"}})
}
pub(super) fn preparation() -> Value {
    json!({"proposal":{"subject":{"operation":"keep","reference":reference()},
        "participant_id":"33333333-3333-4333-8333-333333333333",
        "expected_participant_revision":0,
        "values":{"subject":reference(),"directory_status":"active","role":role()}},
        "review":{"directory_stamp":"44".repeat(32),"selection_reason":"Same represented identity","different":[]},
        "certificate_base64":"AQIDBA=="})
}

#[test]
fn preparation_forwards_exact_proposal_review_and_public_certificate() {
    let input: Preparation = serde_json::from_value(preparation()).unwrap();
    let value = input.validate().unwrap();
    assert_eq!(value.certificate.unwrap(), [1, 2, 3, 4]);
    assert_eq!(value.proposal.expected_participant().get(), 0);
    assert_eq!(value.proposal.values().subject().revision.get(), 1);
    assert_eq!(value.review.directory_stamp.0.to_hex(), "44".repeat(32));
}

#[test]
fn incoherent_subject_refs_and_oversized_review_fail_before_the_workflow() {
    let mut wrong = preparation();
    wrong["proposal"]["values"]["subject"]["revision"] = json!(2);
    assert!(serde_json::from_value::<Preparation>(wrong)
        .unwrap()
        .validate()
        .is_err());
    let mut oversized = preparation();
    let decision = json!({"candidate":{"kind":"subject","id":"22222222-2222-4222-8222-222222222222","revision":1},
        "reason":"Different person","support":role()["role_support"]});
    oversized["review"]["different"] = json!(vec![decision; 17]);
    assert!(serde_json::from_value::<Preparation>(oversized)
        .unwrap()
        .validate()
        .is_err());
}

#[test]
fn base64_requires_standard_padding_and_bounded_decoded_public_material() {
    for encoded in [
        "AQIDBA".to_owned(),
        "AQIDBA=== ".into(),
        "_w==".into(),
        "A".repeat(21848),
        "".into(),
    ] {
        let mut value = preparation();
        value["certificate_base64"] = json!(encoded);
        assert!(serde_json::from_value::<Preparation>(value)
            .unwrap()
            .validate()
            .is_err());
    }
    let request = json!({"subject":{"operation":"keep","reference":reference()},
        "participant":{"operation":"create"},"role":role(),"certificate_base64":"AQIDBA=="});
    assert_eq!(
        serde_json::from_value::<ReviewRequest>(request)
            .unwrap()
            .validate()
            .unwrap()
            .certificate
            .unwrap(),
        [1, 2, 3, 4]
    );
}

#[test]
fn signature_requires_exact_rsa_3072_bytes_and_no_client_verification_result() {
    let value = json!({"prepared":preparation(),"signature_base64":"A".repeat(512)});
    assert_eq!(
        serde_json::from_value::<Submission>(value.clone())
            .unwrap()
            .validate()
            .unwrap()
            .signature
            .unwrap()
            .as_bytes()
            .len(),
        384
    );
    let mut short = value.clone();
    short["signature_base64"] = json!("AQIDBA==");
    assert!(serde_json::from_value::<Submission>(short)
        .unwrap()
        .validate()
        .is_err());
    let mut forged = value;
    forged["check"] = json!({"valid":true});
    assert!(serde_json::from_value::<Submission>(forged).is_err());
}
