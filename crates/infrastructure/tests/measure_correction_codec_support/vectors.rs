use crate::*;

#[test]
fn the_minimum_frame_matches_independent_mcval1_bytes() {
    let bytes = unhex(concat!(
        "4d4356414c31",
        "0000000143",
        "00000012",
        "4d56414c31",
        "00010000000155",
        "0000000156",
        "00",
        "0000000153",
    ));
    assert_eq!(bytes.len(), 38);
    assert_eq!(values(&bytes, &view(&correction())).unwrap(), correction());
    assert_eq!(correction().canonical_bytes(), bytes);
}

#[test]
fn utf8_and_nested_validity_length_match_the_independent_date_frame() {
    let value = MeasureCorrectionValues::new(
        note("C\u{e9}"),
        MeasureValidity::new(
            known(Declared::date("2026-10-04".parse().unwrap(), None).unwrap()),
            note("V"),
            None,
        )
        .unwrap(),
        note("S"),
    );
    let bytes = unhex(concat!(
        "4d4356414c31",
        "0000000343c3a9",
        "00000012",
        "4d56414c31",
        "0107ea0a040000",
        "0000000156",
        "00",
        "0000000153",
    ));
    assert_eq!(bytes.len(), 40);
    assert_eq!(values(&bytes, &view(&value)).unwrap(), value);
    assert_eq!(value.canonical_bytes(), bytes);
}

#[test]
fn the_maximum_frame_contains_five_independent_four_thousand_byte_notes() {
    let text = "\u{1f642}".repeat(1000);
    let value = MeasureCorrectionValues::new(
        note(&text),
        MeasureValidity::new(unknown(&text), note(&text), Some(unknown(&text))).unwrap(),
        note(&text),
    );
    let mut expected = b"MCVAL1".to_vec();
    expected.extend_from_slice(&[0, 0, 15, 160]);
    expected.extend_from_slice(text.as_bytes());
    expected.extend_from_slice(&[0, 0, 46, 246]);
    expected.extend_from_slice(b"MVAL1");
    expected.extend_from_slice(&[0, 1, 0, 0, 15, 160]);
    expected.extend_from_slice(text.as_bytes());
    expected.extend_from_slice(&[0, 0, 15, 160]);
    expected.extend_from_slice(text.as_bytes());
    expected.extend_from_slice(&[1, 0, 1, 0, 0, 15, 160]);
    expected.extend_from_slice(text.as_bytes());
    expected.extend_from_slice(&[0, 0, 15, 160]);
    expected.extend_from_slice(text.as_bytes());

    assert_eq!(expected.len(), 20_040);
    assert_eq!(value.canonical_bytes(), expected);
    let projection = view(&value);
    assert!(serde_json::to_vec(&projection).unwrap().len() <= 32_768);
    assert_eq!(values(&expected, &projection).unwrap(), value);
}
