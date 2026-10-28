use super::{canonical, parse, ErrorKind, Limits, Value, MAX_CANONICAL_INTEGER};

const LIMITS: Limits = Limits { max_values: 1000 };

/// Spell JSON escapes with `~` in place of the backslash.
fn esc(text: &str) -> Vec<u8> {
    text.replace('~', "\\").into_bytes()
}

fn canonical_of(text: &[u8]) -> Vec<u8> {
    let value = parse(text, LIMITS).expect("document is admitted");
    canonical(&value).expect("value is canonical")
}

#[test]
fn rfc8785_sorts_keys_by_utf16_code_units() {
    // The RFC 8785 sorting example: an emoji (a surrogate pair in UTF-16) sorts
    // before U+FB33 although its code point is larger.
    let input = esc(r#"{
        "~u20ac": "Euro Sign",
        "~r": "Carriage Return",
        "~ufb33": "Hebrew Letter Dalet With Dagesh",
        "1": "One",
        "~ud83d~ude00": "Emoji: Grinning Face",
        "~u0080": "Control",
        "~u00f6": "Latin Small Letter O With Diaeresis"
    }"#);
    let expected = [
        r#"{"\r":"Carriage Return","1":"One",""#,
        "\u{80}",
        r#"":"Control",""#,
        "\u{f6}",
        r#"":"Latin Small Letter O With Diaeresis",""#,
        "\u{20ac}",
        r#"":"Euro Sign",""#,
        "\u{1f600}",
        r#"":"Emoji: Grinning Face",""#,
        "\u{fb33}",
        r#"":"Hebrew Letter Dalet With Dagesh"}"#,
    ]
    .concat();
    assert_eq!(canonical_of(&input), expected.into_bytes());
}

#[test]
fn rfc8785_escapes_strings_minimally() {
    // The string member of the RFC 8785 sample object.
    let input = esc(r#"{"string": "~u20ac$~u000F~u000aA'~u0042~u0022~u005c~~~"~/"}"#);
    let expected = [r#"{"string":""#, "\u{20ac}", r#"$\u000f\nA'B\"\\\\\"/"}"#].concat();
    assert_eq!(canonical_of(&input), expected.into_bytes());
}

#[test]
fn escapes_every_control_character_with_short_forms_where_defined() {
    let text: String = (0u8..0x20).map(char::from).collect();
    let mut expected = String::from("\"");
    for byte in 0u8..0x20 {
        match byte {
            0x08 => expected.push_str("\\b"),
            0x09 => expected.push_str("\\t"),
            0x0a => expected.push_str("\\n"),
            0x0c => expected.push_str("\\f"),
            0x0d => expected.push_str("\\r"),
            other => expected.push_str(&format!("\\u{other:04x}")),
        }
    }
    expected.push('"');
    assert_eq!(
        canonical(&Value::String(text)).unwrap(),
        expected.into_bytes()
    );
}

#[test]
fn leaves_delete_solidus_and_line_separators_literal() {
    let text = "/\u{7f}\u{2028}\u{2029}\u{10ffff}".to_owned();
    let expected = format!("\"{text}\"");
    assert_eq!(
        canonical(&Value::String(text)).unwrap(),
        expected.into_bytes()
    );
}

#[test]
fn removes_whitespace_and_sorts_nested_objects() {
    let input = br#" { "b" : [ 3 , { "y" : 1 , "x" : 2 } ] , "a" : 0 , "aa" : "" , "B" : [] } "#;
    assert_eq!(
        canonical_of(input),
        br#"{"B":[],"a":0,"aa":"","b":[3,{"x":2,"y":1}]}"#.to_vec()
    );
}

#[test]
fn canonical_output_parses_back_to_the_same_bytes() {
    let input = esc(r#"{"z":{"k":["~u0000",9007199254740991]},"m":"~ud83d~ude00"}"#);
    let first = canonical_of(&input);
    assert_eq!(canonical_of(&first), first);
}

#[test]
fn integers_are_decimal_within_zero_to_two_to_the_53_minus_one() {
    assert_eq!(canonical(&Value::Integer(0)).unwrap(), b"0".to_vec());
    assert_eq!(
        canonical(&Value::Integer(MAX_CANONICAL_INTEGER)).unwrap(),
        b"9007199254740991".to_vec()
    );
    for outside in [-1, MAX_CANONICAL_INTEGER + 1, i128::MIN, i128::MAX] {
        let error = canonical(&Value::Array(vec![Value::Integer(outside)]))
            .expect_err("integer outside the canonical range");
        assert_eq!(error.kind, ErrorKind::IntegerOutOfRange);
    }
}

#[test]
fn rejects_duplicate_keys_in_a_constructed_object() {
    let value = Value::Object(vec![
        ("a".into(), Value::Integer(1)),
        ("a".into(), Value::Integer(2)),
    ]);
    assert_eq!(canonical(&value).unwrap_err().kind, ErrorKind::DuplicateKey);
}

#[test]
fn empty_containers_and_strings() {
    assert_eq!(canonical_of(b"[ ]"), b"[]".to_vec());
    assert_eq!(canonical_of(b"{ }"), b"{}".to_vec());
    assert_eq!(canonical_of(br#"[ "" , [ ] ]"#), br#"["",[]]"#.to_vec());
}
