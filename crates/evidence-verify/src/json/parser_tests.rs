use super::{parse, ErrorKind, Limits, Value, MAX_DEPTH};

const LIMITS: Limits = Limits { max_values: 1000 };

fn ok(text: &[u8]) -> Value {
    parse(text, LIMITS).expect("document is admitted")
}

fn kind(text: &[u8]) -> ErrorKind {
    parse(text, LIMITS).expect_err("document is rejected").kind
}

/// Spell JSON escapes with `~` in place of the backslash.
fn esc(text: &str) -> Vec<u8> {
    text.replace('~', "\\").into_bytes()
}

fn string(text: &str) -> Value {
    Value::String(text.to_owned())
}

#[test]
fn parses_the_admitted_subset_keeping_member_order() {
    let value = ok(br#" { "b" : [1, -2, "x"], "a" : {}, "c" : [] } "#);
    assert_eq!(
        value,
        Value::Object(vec![
            (
                "b".into(),
                Value::Array(vec![Value::Integer(1), Value::Integer(-2), string("x")])
            ),
            ("a".into(), Value::Object(vec![])),
            ("c".into(), Value::Array(vec![])),
        ])
    );
    assert_eq!(value.get("a"), Some(&Value::Object(vec![])));
    assert_eq!(value.get("z"), None);
}

#[test]
fn admits_scalars_at_the_top_level_and_json_whitespace() {
    assert_eq!(ok(b"0"), Value::Integer(0));
    assert_eq!(ok(b"\t\r\n 7 \n"), Value::Integer(7));
    assert_eq!(ok(br#""""#), string(""));
}

#[test]
fn integers_cover_i64_minimum_to_u64_maximum() {
    assert_eq!(
        ok(b"18446744073709551615"),
        Value::Integer(u64::MAX as i128)
    );
    assert_eq!(
        ok(b"-9223372036854775808"),
        Value::Integer(i64::MIN as i128)
    );
    assert_eq!(kind(b"18446744073709551616"), ErrorKind::IntegerOutOfRange);
    assert_eq!(kind(b"-9223372036854775809"), ErrorKind::IntegerOutOfRange);
    assert_eq!(
        kind(b"1000000000000000000000000000000000000000000"),
        ErrorKind::IntegerOutOfRange
    );
}

#[test]
fn rejects_fractions_exponents_leading_zeros_and_negative_zero() {
    assert_eq!(kind(b"1.0"), ErrorKind::NotInteger);
    assert_eq!(kind(b"1e3"), ErrorKind::NotInteger);
    assert_eq!(kind(b"1E3"), ErrorKind::NotInteger);
    assert_eq!(kind(b"[0.5]"), ErrorKind::NotInteger);
    assert_eq!(kind(b"01"), ErrorKind::LeadingZero);
    assert_eq!(kind(b"-01"), ErrorKind::LeadingZero);
    assert_eq!(kind(b"-0"), ErrorKind::NegativeZero);
    assert_eq!(kind(b"-"), ErrorKind::UnexpectedEnd);
    assert_eq!(kind(b"-a"), ErrorKind::UnexpectedByte);
    assert_eq!(kind(b"+1"), ErrorKind::UnexpectedByte);
}

#[test]
fn rejects_literals() {
    assert_eq!(kind(b"true"), ErrorKind::Literal);
    assert_eq!(kind(b"false"), ErrorKind::Literal);
    assert_eq!(kind(b"[null]"), ErrorKind::Literal);
    assert_eq!(kind(b"nul"), ErrorKind::UnexpectedByte);
}

#[test]
fn rejects_duplicate_keys_including_escaped_spellings() {
    assert_eq!(kind(br#"{"a":1,"a":2}"#), ErrorKind::DuplicateKey);
    assert_eq!(kind(&esc(r#"{"a":1,"~u0061":2}"#)), ErrorKind::DuplicateKey);
    assert_eq!(
        ok(br#"[{"a":1},{"a":2}]"#).as_array().map(<[_]>::len),
        Some(2)
    );
}

#[test]
fn rejects_trailing_data_and_empty_input() {
    assert_eq!(kind(b""), ErrorKind::UnexpectedEnd);
    assert_eq!(kind(b"   "), ErrorKind::UnexpectedEnd);
    assert_eq!(kind(b"{} {}"), ErrorKind::TrailingData);
    assert_eq!(kind(b"1 2"), ErrorKind::TrailingData);
    assert_eq!(kind(b"{}\0"), ErrorKind::TrailingData);
    assert_eq!(kind(b"\xef\xbb\xbf{}"), ErrorKind::UnexpectedByte);
}

#[test]
fn rejects_structural_errors() {
    assert_eq!(kind(b"[1,]"), ErrorKind::UnexpectedByte);
    assert_eq!(kind(br#"{"a":1,}"#), ErrorKind::UnexpectedByte);
    assert_eq!(kind(b"[1 2]"), ErrorKind::UnexpectedByte);
    assert_eq!(kind(br#"{"a" 1}"#), ErrorKind::UnexpectedByte);
    assert_eq!(kind(b"{1:2}"), ErrorKind::UnexpectedByte);
    assert_eq!(kind(b"[1"), ErrorKind::UnexpectedEnd);
    assert_eq!(kind(br#"{"a":"#), ErrorKind::UnexpectedEnd);
    assert_eq!(kind(b"\x0b1"), ErrorKind::UnexpectedByte);
}

#[test]
fn rejects_invalid_utf8() {
    assert_eq!(kind(b"\"\xff\""), ErrorKind::InvalidUtf8);
    assert_eq!(kind(b"\"\xc0\xaf\""), ErrorKind::InvalidUtf8);
    assert_eq!(kind(b"\"\xed\xa0\x80\""), ErrorKind::InvalidUtf8);
}

#[test]
fn decodes_escapes_and_surrogate_pairs() {
    assert_eq!(
        ok(&esc(r#""~"~~~/~b~f~n~r~t~u0041~u00e9~ud83d~ude00""#)),
        string("\"\\/\u{8}\u{c}\n\r\tA\u{e9}\u{1f600}")
    );
    assert_eq!(ok(b"\"\xc3\xa9\xe2\x82\xac\""), string("\u{e9}\u{20ac}"));
}

#[test]
fn rejects_bad_escapes_surrogates_and_control_characters() {
    assert_eq!(kind(br#""\x""#), ErrorKind::InvalidEscape);
    assert_eq!(kind(br#""\u00g0""#), ErrorKind::InvalidEscape);
    assert_eq!(kind(br#""\u00""#), ErrorKind::InvalidEscape);
    assert_eq!(kind(br#""\ud83d""#), ErrorKind::UnpairedSurrogate);
    assert_eq!(kind(br#""\ud83dx""#), ErrorKind::UnpairedSurrogate);
    assert_eq!(
        kind(&esc(r#""~ud83d~u0041""#)),
        ErrorKind::UnpairedSurrogate
    );
    assert_eq!(kind(br#""\ude00""#), ErrorKind::UnpairedSurrogate);
    assert_eq!(kind(b"\"a\nb\""), ErrorKind::ControlCharacter);
    assert_eq!(kind(b"\"\x00\""), ErrorKind::ControlCharacter);
    assert_eq!(kind(b"\"\x1f\""), ErrorKind::ControlCharacter);
    assert_eq!(ok(b"\"\x7f\""), string("\u{7f}"));
    assert_eq!(kind(b"\"abc"), ErrorKind::UnexpectedEnd);
}

#[test]
fn bounds_nesting_depth() {
    let nested = |depth: usize| {
        let mut text = "[".repeat(depth);
        text.push_str(&"]".repeat(depth));
        text
    };
    assert!(parse(nested(MAX_DEPTH).as_bytes(), LIMITS).is_ok());
    assert_eq!(kind(nested(MAX_DEPTH + 1).as_bytes()), ErrorKind::TooDeep);
    let objects = format!(
        "{}1{}",
        r#"{"a":"#.repeat(MAX_DEPTH + 1),
        "}".repeat(MAX_DEPTH + 1)
    );
    assert_eq!(kind(objects.as_bytes()), ErrorKind::TooDeep);
}

#[test]
fn bounds_the_number_of_values() {
    let small = Limits { max_values: 4 };
    assert!(parse(b"[1,2,3]", small).is_ok());
    assert!(parse(br#"{"a":1,"b":[2]}"#, small).is_ok());
    let error = parse(b"[1,2,3,4]", small).expect_err("too many values");
    assert_eq!(error.kind, ErrorKind::TooManyValues);
    assert_eq!(error.code(), "json.too_many_values");
    assert!(parse(b"1", Limits { max_values: 0 }).is_err());
}

#[test]
fn reports_the_offset_of_the_rejection() {
    let error = parse(b"[1, 2, x]", LIMITS).expect_err("bad byte");
    assert_eq!(error.kind, ErrorKind::UnexpectedByte);
    assert_eq!(error.offset, 7);
}

/// An object of `count` distinct keys, optionally ending with a repeat of
/// the first key.
fn wide_object(count: usize, repeat_first: bool) -> Vec<u8> {
    let mut text = String::from("{");
    for index in 0..count {
        text.push_str(&format!("\"k{index}\":0,"));
    }
    if repeat_first {
        text.push_str("\"k0\":0,");
    }
    text.pop();
    text.push('}');
    text.into_bytes()
}

#[test]
fn duplicate_detection_scales_to_wide_objects() {
    // Quadratic duplicate detection would take minutes at this width.
    let count = 200_000;
    let limits = Limits {
        max_values: count + 2,
    };
    let parsed = parse(&wide_object(count, false), limits).expect("distinct keys");
    assert_eq!(parsed.as_object().map(<[_]>::len), Some(count));
    let error = parse(&wide_object(count, true), limits).expect_err("repeated key");
    assert_eq!(error.kind, ErrorKind::DuplicateKey);
}
