use super::{encode, parse, Receipt};
use crate::report::Outcome;

fn receipt() -> Receipt {
    Receipt {
        operation_id: "6f1c2a3b-4d5e-4f60-8a71-b2c3d4e5f604".to_owned(),
        manifest_sha256: [0xab; 32],
        accepted_at_seconds: 1_793_207_656,
        accepted_at_nanoseconds: 123_456_789,
    }
}

fn expected_text() -> String {
    [
        r#"{"accepted_at_nanoseconds":123456789,"accepted_at_seconds":1793207656,"#,
        r#""manifest_sha256":""#,
        &"ab".repeat(32),
        r#"","operation_id":"6f1c2a3b-4d5e-4f60-8a71-b2c3d4e5f604","#,
        r#""profile":"qadra-acceptance-receipt","profile_version":1,"#,
        r#""provenance":"application-record"}"#,
    ]
    .concat()
}

fn failed(code: &str, text: &str) {
    let finding = parse(text.as_bytes()).expect_err("rejected");
    assert_eq!(
        (finding.outcome, finding.code),
        (Outcome::Failed, code),
        "{text}"
    );
}

#[test]
fn encodes_the_seven_members_canonically_and_parses_them_back() {
    let bytes = encode(&receipt()).expect("encodes");
    assert_eq!(String::from_utf8(bytes.clone()).unwrap(), expected_text());
    assert_eq!(parse(&bytes).expect("parses"), receipt());
}

#[test]
fn rejects_non_canonical_bytes() {
    failed("json.not_canonical", &format!("{}\n", expected_text()));
    failed(
        "json.not_canonical",
        &expected_text().replace(":1,", ": 1,"),
    );
}

#[test]
fn rejects_unknown_missing_and_wrong_fixed_members() {
    let text = expected_text();
    failed(
        "receipt.schema",
        &text.replace(r#""provenance""#, r#""provenancf""#),
    );
    failed(
        "receipt.schema",
        &text.replace(r#","provenance":"application-record""#, ""),
    );
    failed(
        "receipt.schema",
        &text.replace("application-record", "token"),
    );
    failed(
        "receipt.schema",
        &text.replace("qadra-acceptance-receipt", "qadra-other"),
    );
    failed(
        "receipt.schema",
        &text.replace(r#""profile_version":1"#, r#""profile_version":2"#),
    );
    failed("receipt.schema", &text.replace("abab", "ABAB"));
    failed("receipt.schema", &text.replace("6f1c2a3b", "6F1C2A3B"));
    failed("receipt.schema", "[]");
}

#[test]
fn nanoseconds_stay_below_one_second() {
    let text = expected_text().replace("123456789", "1000000000");
    failed("receipt.schema", &text);
    let mut edge = receipt();
    edge.accepted_at_nanoseconds = 999_999_999;
    let bytes = encode(&edge).expect("encodes");
    assert_eq!(parse(&bytes).expect("parses"), edge);
}

#[test]
fn encode_refuses_values_the_parser_would_reject() {
    let mut late = receipt();
    late.accepted_at_nanoseconds = 1_000_000_000;
    assert_eq!(encode(&late).unwrap_err().code, "receipt.schema");
    let mut negative = receipt();
    negative.accepted_at_seconds = -1;
    assert_eq!(
        encode(&negative).unwrap_err().code,
        "json.integer_out_of_range"
    );
}
