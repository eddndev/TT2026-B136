use super::*;

fn from_hex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn review_decodes_an_independent_phear1_literal() {
    let expected = from_hex(concat!(
        "50484541523101",
        "0000000000000000",
        "00000000",
        "00",
        "00000007436f7572742041",
        "01000000044e6f7465",
        "01",
        "00000000000000000000000000000009",
        "00000002",
        "0000000c536574206279206f72646572",
        "0000000000000000000000000000004d",
        "00000004",
        "4242424242424242424242424242424242424242424242424242424242424242",
        "00000006506167652032",
        "01",
        "0000000000000000000000000000000b",
        "00000003",
        "5555555555555555555555555555555555555555555555555555555555555555"
    ));
    let original = fixture(true);
    assert_eq!(original.canonical_bytes(), expected);
    assert_eq!(values(&expected, &view(&original)).unwrap(), original);
}

fn text(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u32).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

#[test]
fn complete_utf8_maxima_match_independent_bytes_and_exact_size_limits() {
    let scalar = "\u{10000}";
    let venue = scalar.repeat(500);
    let note = scalar.repeat(1000);
    for review in [false, true] {
        let mut source = input(review);
        source.venue = HearingVenue::new(&venue).unwrap();
        source.note = Some(HearingNote::new(&note).unwrap());
        source.participants = (1..=32).map(participant).collect();
        source.review_targets = if review {
            (1..=32).map(target).collect()
        } else {
            vec![]
        };
        source.scheduling_basis = PrecautionaryHearingSchedulingBasis::new(
            HearingNote::new(&note).unwrap(),
            source.scheduling_basis.support(),
            HearingNote::new(&note).unwrap(),
        );
        let original = PrecautionaryHearingValues::new(source).unwrap();

        let mut expected = b"PHEAR1".to_vec();
        expected.push(u8::from(review));
        expected.extend_from_slice(&[0; 13]);
        text(&mut expected, &venue);
        expected.push(1);
        text(&mut expected, &note);
        expected.push(32);
        for id in 1u128..=32 {
            expected.extend_from_slice(&id.to_be_bytes());
            expected.extend_from_slice(&2u32.to_be_bytes());
        }
        text(&mut expected, &note);
        expected.extend_from_slice(&77u128.to_be_bytes());
        expected.extend_from_slice(&4u32.to_be_bytes());
        expected.extend_from_slice(&[0x42; 32]);
        text(&mut expected, &note);
        expected.push(if review { 32 } else { 0 });
        if review {
            for id in 1u128..=32 {
                expected.extend_from_slice(&id.to_be_bytes());
                expected.extend_from_slice(&3u32.to_be_bytes());
                expected.extend_from_slice(&[0x55; 32]);
            }
        }
        assert_eq!(expected.len(), if review { 16_395 } else { 14_731 });
        assert_eq!(original.canonical_bytes(), expected);
        let projection = view(&original);
        assert!(serde_json::to_vec(&projection).unwrap().len() <= 65_536);
        assert_eq!(values(&expected, &projection).unwrap(), original);
    }
}
