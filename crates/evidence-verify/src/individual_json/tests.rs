use super::{parse, IndividualSigning};
use crate::report::Outcome;

/// One member of a test document: a key and its JSON text.
type Member = (String, String);

fn bytes(length: usize, fill: u8) -> String {
    let items: Vec<String> = (0..length).map(|_| fill.to_string()).collect();
    format!("[{}]", items.join(","))
}

/// Members in the order and compact form the server writes them.
fn members() -> Vec<Member> {
    [
        ("format_version", "1".to_owned()),
        ("binding_statement", bytes(142, 1)),
        ("binding_signature", bytes(384, 2)),
        ("binding_root_der", bytes(3, 3)),
        ("binding_crl_der", bytes(2, 4)),
        ("binding_registered_at", "1700000000".to_owned()),
        ("consent_statement", bytes(258, 5)),
        ("consent_signature", bytes(384, 6)),
        ("accepted_at", "1700000100".to_owned()),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_owned(), value))
    .collect()
}

fn document(members: &[Member]) -> Vec<u8> {
    let body: Vec<String> = members
        .iter()
        .map(|(key, value)| format!("\"{key}\":{value}"))
        .collect();
    format!("{{{}}}", body.join(",")).into_bytes()
}

fn with(key: &str, value: &str) -> Vec<u8> {
    let mut list = members();
    list.iter_mut()
        .find(|(name, _)| name == key)
        .expect("known key")
        .1 = value.to_owned();
    document(&list)
}

fn without(key: &str) -> Vec<u8> {
    let list: Vec<Member> = members()
        .into_iter()
        .filter(|(name, _)| name != key)
        .collect();
    document(&list)
}

fn rejected(input: &[u8]) -> (Outcome, &'static str) {
    let finding = parse(input).expect_err("document is rejected");
    (finding.outcome, finding.code)
}

#[test]
fn parses_the_server_layout() {
    let parsed = parse(&document(&members())).expect("server layout is admitted");
    assert_eq!(
        parsed,
        IndividualSigning {
            format_version: 1,
            binding_statement: vec![1; 142],
            binding_signature: vec![2; 384],
            binding_root_der: vec![3; 3],
            binding_crl_der: vec![4; 2],
            binding_registered_at: 1_700_000_000,
            consent_statement: vec![5; 258],
            consent_signature: vec![6; 384],
            accepted_at: 1_700_000_100,
        }
    );
}

#[test]
fn accepts_any_member_order_and_whitespace() {
    let mut list = members();
    list.reverse();
    let mut text = document(&list);
    text.insert(1, b'\n');
    text.push(b'\n');
    assert!(parse(&text).is_ok());
}

#[test]
fn rejects_unknown_missing_and_duplicate_keys() {
    let mut extra = members();
    extra.push(("comment".into(), "\"x\"".into()));
    assert_eq!(
        rejected(&document(&extra)),
        (Outcome::Failed, "individual_json.unknown_key")
    );
    assert_eq!(
        rejected(&without("consent_signature")),
        (Outcome::Failed, "individual_json.missing_key")
    );
    let mut duplicate = members();
    duplicate.push(("accepted_at".into(), "1700000100".into()));
    assert_eq!(
        rejected(&document(&duplicate)),
        (Outcome::Failed, "individual_json.malformed")
    );
}

#[test]
fn rejects_malformed_json_and_non_objects() {
    for input in [&b""[..], b"{", b"[]", b"1", b"{} {}", b"\xff"] {
        let (outcome, code) = rejected(input);
        assert_eq!(outcome, Outcome::Failed);
        assert!(
            code == "individual_json.malformed" || code == "individual_json.not_object",
            "{code}"
        );
    }
    assert_eq!(rejected(b"[]").1, "individual_json.not_object");
    assert_eq!(rejected(b"{").1, "individual_json.malformed");
}

#[test]
fn requires_format_version_one() {
    assert_eq!(
        rejected(&with("format_version", "2")),
        (Outcome::NotSupported, "individual_json.format_version")
    );
    assert_eq!(
        rejected(&with("format_version", "0")),
        (Outcome::NotSupported, "individual_json.format_version")
    );
    assert_eq!(
        rejected(&with("format_version", "\"1\"")),
        (Outcome::Failed, "individual_json.type")
    );
    assert_eq!(
        rejected(&with("format_version", "1.0")),
        (Outcome::Failed, "individual_json.malformed")
    );
}

#[test]
fn byte_fields_are_arrays_of_integers_up_to_255() {
    let mut root = bytes(2, 0);
    root.insert_str(root.len() - 1, ",255");
    assert_eq!(
        parse(&with("binding_root_der", &root))
            .unwrap()
            .binding_root_der,
        vec![0, 0, 255]
    );
    for bad in ["[256]", "[-1]", "[1,256]"] {
        assert_eq!(
            rejected(&with("binding_root_der", bad)),
            (Outcome::Failed, "individual_json.byte_range")
        );
    }
    for bad in ["\"AAEC\"", "[\"1\"]", "[[1]]", "{}", "1"] {
        assert_eq!(
            rejected(&with("binding_crl_der", bad)),
            (Outcome::Failed, "individual_json.type"),
            "{bad}"
        );
    }
}

#[test]
fn statements_and_signatures_have_exact_lengths() {
    for (key, length) in [
        ("binding_statement", 142),
        ("binding_signature", 384),
        ("consent_statement", 258),
        ("consent_signature", 384),
    ] {
        for wrong in [0, length - 1, length + 1] {
            assert_eq!(
                rejected(&with(key, &bytes(wrong, 0))),
                (Outcome::Failed, "individual_json.length"),
                "{key} {wrong}"
            );
        }
    }
}

#[test]
fn root_and_crl_are_non_empty_and_bounded() {
    for key in ["binding_root_der", "binding_crl_der"] {
        assert_eq!(
            rejected(&with(key, "[]")),
            (Outcome::Failed, "individual_json.length")
        );
    }
    assert_eq!(
        parse(&with("binding_root_der", &bytes(16384, 7)))
            .unwrap()
            .binding_root_der
            .len(),
        16384
    );
    assert_eq!(
        rejected(&with("binding_root_der", &bytes(16385, 7))),
        (Outcome::Failed, "individual_json.length")
    );
}

#[test]
fn crl_admits_one_mebibyte_and_no_more() {
    let largest = with("binding_crl_der", &bytes(1024 * 1024, 255));
    assert!(largest.len() <= 4_718_592);
    assert_eq!(parse(&largest).unwrap().binding_crl_der.len(), 1024 * 1024);
    assert_eq!(
        rejected(&with("binding_crl_der", &bytes(1024 * 1024 + 1, 255))),
        (Outcome::Failed, "individual_json.length")
    );
}

#[test]
fn instants_cover_the_years_the_server_can_represent() {
    let parsed = parse(&with("binding_registered_at", "-377705116800")).unwrap();
    assert_eq!(parsed.binding_registered_at, -377_705_116_800);
    let parsed = parse(&with("accepted_at", "253402300799")).unwrap();
    assert_eq!(parsed.accepted_at, 253_402_300_799);
    for (key, value) in [
        ("binding_registered_at", "-377705116801"),
        ("binding_registered_at", "-9223372036854775808"),
        ("accepted_at", "253402300800"),
        ("accepted_at", "9223372036854775807"),
        ("accepted_at", "9223372036854775808"),
    ] {
        assert_eq!(
            rejected(&with(key, value)),
            (Outcome::Failed, "individual_json.instant_range"),
            "{key} = {value}"
        );
    }
    assert_eq!(
        rejected(&with("accepted_at", "\"1700000100\"")),
        (Outcome::Failed, "individual_json.type")
    );
}

#[test]
fn instant_order_is_left_to_the_context_check() {
    let parsed = parse(&with("accepted_at", "1")).unwrap();
    assert!(parsed.accepted_at < parsed.binding_registered_at);
}

#[test]
fn rejects_documents_larger_than_the_entry_limit() {
    let mut text = document(&members());
    text.resize(4_718_593, b' ');
    assert_eq!(
        rejected(&text),
        (Outcome::NotSupported, "individual_json.too_large")
    );
}
