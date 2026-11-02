use super::{
    encode_partial, encode_receipt, parse_partial, parse_receipt, ContractReceipt, PartialReceipt,
};
use crate::report::{Finding, Outcome};

fn receipt() -> ContractReceipt {
    ContractReceipt {
        request_id: "10000000-0000-4000-8000-000000000003".to_owned(),
        request_sha256: [0xcd; 32],
        ceremony_reference: "HOJA-2026-0001".to_owned(),
        client_accepted_at: 1_800_000_100,
        firm_accepted_at: 1_800_000_200,
        completed_at: 1_800_000_201,
    }
}

fn partial() -> PartialReceipt {
    PartialReceipt {
        request_id: "10000000-0000-4000-8000-000000000003".to_owned(),
        request_sha256: [0xcd; 32],
        binding_registered_at: 1_799_000_000,
        accepted_at: 1_800_000_100,
        ceremony_reference: "HOJA-2026-0001".to_owned(),
        exported_at: 1_800_000_500,
    }
}

fn receipt_text() -> String {
    [
        r#"{"ceremony_reference":"HOJA-2026-0001","client_accepted_at":1800000100,"#,
        r#""completed_at":1800000201,"firm_accepted_at":1800000200,"#,
        r#""provenance":"application-record","#,
        r#""request_id":"10000000-0000-4000-8000-000000000003","request_sha256":""#,
        &"cd".repeat(32),
        r#""}"#,
    ]
    .concat()
}

fn partial_text() -> String {
    [
        r#"{"accepted_at":1800000100,"binding_registered_at":1799000000,"#,
        r#""ceremony_reference":"HOJA-2026-0001","exported_at":1800000500,"#,
        r#""provenance":"application-record","#,
        r#""request_id":"10000000-0000-4000-8000-000000000003","request_sha256":""#,
        &"cd".repeat(32),
        r#""}"#,
    ]
    .concat()
}

fn code(result: Result<impl core::fmt::Debug, Finding>) -> (Outcome, &'static str) {
    let finding = result.expect_err("rejected");
    (finding.outcome, finding.code)
}

#[test]
fn both_receipts_encode_canonically_and_parse_back() {
    let bytes = encode_receipt(&receipt()).expect("encodes");
    assert_eq!(String::from_utf8(bytes.clone()).unwrap(), receipt_text());
    assert_eq!(parse_receipt(&bytes).expect("parses"), receipt());
    let bytes = encode_partial(&partial()).expect("encodes");
    assert_eq!(String::from_utf8(bytes.clone()).unwrap(), partial_text());
    assert_eq!(parse_partial(&bytes).expect("parses"), partial());
}

#[test]
fn each_receipt_has_its_own_schema_code() {
    let full = (Outcome::Failed, "receipt.contract_schema");
    let acknowledgement = (Outcome::Failed, "receipt.contract_partial_schema");
    assert_eq!(code(parse_receipt(partial_text().as_bytes())), full);
    assert_eq!(
        code(parse_partial(receipt_text().as_bytes())),
        acknowledgement
    );
    let text = partial_text().replace("application-record", "application-recorc");
    assert_eq!(code(parse_partial(text.as_bytes())), acknowledgement);
    let text = receipt_text().replace("1800000201", r#""1800000201""#);
    assert_eq!(code(parse_receipt(text.as_bytes())), full);
    let text = receipt_text().replace("10000000-0000", "1000000G-0000");
    assert_eq!(code(parse_receipt(text.as_bytes())), full);
    let text = partial_text().replace(&"cd".repeat(32), &"CD".repeat(32));
    assert_eq!(code(parse_partial(text.as_bytes())), acknowledgement);
}

#[test]
fn the_ceremony_reference_holds_one_to_eighty_characters() {
    let schema = (Outcome::Failed, "receipt.contract_partial_schema");
    let text = partial_text().replace("HOJA-2026-0001", "");
    assert_eq!(code(parse_partial(text.as_bytes())), schema);
    let long = "x".repeat(81);
    let text = partial_text().replace("HOJA-2026-0001", &long);
    assert_eq!(code(parse_partial(text.as_bytes())), schema);
    let mut edge = partial();
    edge.ceremony_reference = "x".repeat(80);
    let bytes = encode_partial(&edge).expect("80 characters");
    assert_eq!(parse_partial(&bytes).expect("parses"), edge);
    let mut empty = receipt();
    empty.ceremony_reference.clear();
    assert_eq!(
        code(encode_receipt(&empty)),
        (Outcome::Failed, "receipt.contract_schema")
    );
}

#[test]
fn rejects_non_canonical_bytes() {
    let text = format!("{}\n", receipt_text());
    assert_eq!(
        code(parse_receipt(text.as_bytes())),
        (Outcome::Failed, "json.not_canonical")
    );
}
