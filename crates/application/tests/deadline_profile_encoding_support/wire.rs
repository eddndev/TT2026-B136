use super::deadline_profile_encoding_support::*;
use application::deadline_profiles::*;
use domain::{
    deadline_arithmetic::{ArithmeticOutcome, ArithmeticRule},
    deadline_profiles::{DeadlineRuleTemplate, OrderedDeadlineUnit},
    procedural_time::DeclaredProceduralTime,
};
use time::UtcOffset;

fn u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}
fn text(bytes: &mut Vec<u8>, value: &[u8]) {
    u32(bytes, value.len() as u32);
    bytes.extend_from_slice(value);
}
fn minimum() -> Vec<u8> {
    let mut bytes = b"DPRF1".to_vec();
    text(&mut bytes, b"x");
    text(&mut bytes, b"x");
    bytes.push(1);
    bytes.extend_from_slice(&[0; 16]);
    u32(&mut bytes, 1);
    bytes.extend_from_slice(&[0; 16]);
    text(&mut bytes, b"x");
    text(&mut bytes, b"x");
    text(&mut bytes, b"https://a.aa");
    bytes.push(0);
    bytes.extend_from_slice(&0_i32.to_be_bytes());
    text(&mut bytes, b"x");
    bytes.extend_from_slice(&[0, 0, 0, 1]);
    u32(&mut bytes, 1);
    bytes.extend_from_slice(&[0, 1]);
    u32(&mut bytes, 1);
    bytes.extend_from_slice(&[0; 16]);
    text(&mut bytes, b"x");
    u32(&mut bytes, 1);
    bytes.extend_from_slice(&[0; 16]);
    u32(&mut bytes, 1);
    bytes.extend_from_slice(&[0; 16]);
    bytes.extend_from_slice(&[1, 7, 178, 1, 1, 0, 0, 0, 0, 0]);
    bytes.extend_from_slice(&31_i32.to_be_bytes());
    u32(&mut bytes, 1);
    bytes.extend_from_slice(&[0; 16]);
    text(&mut bytes, b"x");
    bytes
}
fn rejected(bytes: &[u8]) {
    assert!(
        decode_deadline_profile_definition(bytes).is_err(),
        "accepted malformed profile of {} bytes",
        bytes.len()
    );
}

#[test]
fn independent_minimum_vector_fixes_layout_and_rejects_every_truncation_and_trailing_byte() {
    let bytes = minimum();
    assert_eq!(bytes.len(), 202);
    let expected = DeadlineProfileDefinition::new(input()).unwrap();
    assert_eq!(deadline_profile_definition_bytes(&expected), bytes);
    assert_eq!(
        decode_deadline_profile_definition(&bytes).unwrap(),
        expected
    );
    for length in 0..bytes.len() {
        rejected(&bytes[..length]);
    }
    let mut trailing = bytes;
    trailing.push(0);
    rejected(&trailing);
    rejected(&vec![0; 3261698]);
}

#[test]
fn invalid_prefix_discriminants_quantities_and_collection_counts_fail() {
    let original = minimum();
    for position in 0..5 {
        let mut bytes = original.clone();
        bytes[position] ^= 1;
        rejected(&bytes);
    }
    for (position, invalid) in [
        (15, 2),
        (78, 2),
        (88, 2),
        (89, 5),
        (90, 2),
        (91, 3),
        (96, 2),
        (97, 3),
        (163, 4),
        (168, 2),
        (169, 2),
        (170, 2),
        (171, 2),
        (172, 3),
    ] {
        let mut bytes = original.clone();
        bytes[position] = invalid;
        rejected(&bytes);
    }
    for position in [32, 98, 123, 143, 177] {
        for count in [0_u32, 17, u32::MAX] {
            let mut bytes = original.clone();
            bytes[position..position + 4].copy_from_slice(&count.to_be_bytes());
            rejected(&bytes);
        }
    }
    let mut zero_quantity = original;
    zero_quantity[92..96].fill(0);
    rejected(&zero_quantity);
}

#[test]
fn invalid_utf8_text_lengths_urls_dates_and_alternative_normalization_fail() {
    for position in [5, 10, 52, 57, 62, 83, 118, 197] {
        let mut bytes = minimum();
        bytes[position..position + 4].copy_from_slice(&u32::MAX.to_be_bytes());
        rejected(&bytes);
    }
    for position in [9, 14, 56, 61, 87, 122, 201] {
        for value in [255, 0, b' ', b'\n'] {
            let mut bytes = minimum();
            bytes[position] = value;
            rejected(&bytes);
        }
    }
    let mut bad_url = minimum();
    bad_url[66] = b'f';
    rejected(&bad_url);
    let mut invalid_civil = minimum();
    invalid_civil[79..83].copy_from_slice(&i32::MAX.to_be_bytes());
    rejected(&invalid_civil);
    for (position, replacement) in [(164, vec![0, 0]), (166, vec![13]), (167, vec![0])] {
        let mut bytes = minimum();
        bytes[position..position + replacement.len()].copy_from_slice(&replacement);
        rejected(&bytes);
    }
    let mut normalized_title = minimum();
    normalized_title[5..9].copy_from_slice(&2_u32.to_be_bytes());
    normalized_title.insert(9, b' ');
    rejected(&normalized_title);
}

#[test]
fn decoder_replays_the_corpus_and_validates_reference_links_and_completion() {
    let mut mismatch = minimum();
    mismatch[173..177].copy_from_slice(&32_i32.to_be_bytes());
    assert!(matches!(
        decode_deadline_profile_definition(&mismatch),
        Err(DeadlineProfileError::ExampleMismatch(_))
    ));
    for position in [127, 181] {
        let mut foreign = minimum();
        foreign[position] = 1;
        rejected(&foreign);
    }
    let mut incompatible = minimum();
    incompatible[97] = 0;
    rejected(&incompatible);
    let mut only_blocked = minimum();
    only_blocked.splice(163..169, [0]);
    only_blocked.splice(166..172, [0, 2, 0]);
    only_blocked[5..9].copy_from_slice(&9_u32.to_be_bytes());
    only_blocked.splice(9..10, b"synthetic".iter().copied());
    assert_eq!(only_blocked.len(), 202);
    assert!(matches!(
        decode_deadline_profile_definition(&only_blocked),
        Err(DeadlineProfileError::NoSuccessfulExample)
    ));
}

#[test]
fn duplicate_collection_id_or_link_is_rejected_instead_of_deduplicated() {
    for (count_at, start, end) in [
        (32, 36, 88),
        (98, 102, 143),
        (143, 147, 202),
        (123, 127, 143),
        (177, 181, 197),
    ] {
        let mut bytes = minimum();
        let repeated = bytes[start..end].to_vec();
        bytes[count_at..count_at + 4].copy_from_slice(&2_u32.to_be_bytes());
        bytes.splice(end..end, repeated);
        rejected(&bytes);
    }
}

#[test]
fn ordered_maximum_quantity_and_unit_tags_are_validated_before_reproduction() {
    let mut values = input();
    values.template = DeadlineRuleTemplate::Ordered {
        unit: OrderedDeadlineUnit::ElapsedHours,
        maximum: Some(quantity(2)),
    };
    values.completion = DeadlineCompletionPolicy::ArithmeticInstant;
    values.examples[0].anchor =
        DeclaredProceduralTime::second(date("2026-01-06"), 0, 0, 0, Some(UtcOffset::UTC)).unwrap();
    values.examples[0].ordered_quantity = Some(quantity(1));
    recalculate(&mut values);
    let bytes = roundtrip(values);
    assert_eq!(&bytes[90..97], &[1, 2, 1, 0, 0, 0, 2]);
    for (position, replacement) in [(91, vec![3]), (92, vec![2]), (93, vec![0, 0, 0, 0])] {
        let mut invalid = bytes.clone();
        invalid[position..position + replacement.len()].copy_from_slice(&replacement);
        rejected(&invalid);
    }
}

#[test]
fn expected_instant_nanoseconds_and_offset_are_not_silently_normalized() {
    let mut values = input();
    values.template = DeadlineRuleTemplate::Fixed(ArithmeticRule::ElapsedHours {
        quantity: quantity(1),
    });
    values.completion = DeadlineCompletionPolicy::ArithmeticInstant;
    values.examples[0].anchor =
        DeclaredProceduralTime::second(date("2026-01-06"), 0, 0, 0, Some(UtcOffset::UTC)).unwrap();
    recalculate(&mut values);
    let DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::InstantCandidate { instant }) =
        values.examples[0].expected
    else {
        unreachable!()
    };
    let bytes = roundtrip(values);
    let pattern = [
        vec![0, 1],
        instant.unix_timestamp().to_be_bytes().to_vec(),
        vec![0; 8],
    ]
    .concat();
    let position = bytes
        .windows(pattern.len())
        .position(|part| part == pattern)
        .unwrap();
    for nanos in [1_u32, 1_000_000_000, u32::MAX] {
        let mut invalid = bytes.clone();
        invalid[position + 10..position + 14].copy_from_slice(&nanos.to_be_bytes());
        rejected(&invalid);
    }
    for seconds in [-93599_i32, -86400, 86400, 93599] {
        let mut valid = bytes.clone();
        valid[position + 14..position + 18].copy_from_slice(&seconds.to_be_bytes());
        let decoded = decode_deadline_profile_definition(&valid).unwrap();
        let DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::InstantCandidate {
            instant: actual,
        }) = decoded.examples()[0].expected
        else {
            unreachable!()
        };
        assert_eq!(
            actual.unix_timestamp_nanos(),
            instant.unix_timestamp_nanos()
        );
        assert_eq!(actual.offset().whole_seconds(), seconds);
        assert_eq!(deadline_profile_definition_bytes(&decoded), valid);
    }
    for seconds in [-93600_i32, 93600, i32::MIN, i32::MAX] {
        assert!(UtcOffset::from_whole_seconds(seconds).is_err());
        let mut invalid_offset = bytes.clone();
        invalid_offset[position + 14..position + 18].copy_from_slice(&seconds.to_be_bytes());
        rejected(&invalid_offset);
    }
    let mut invalid_timestamp = bytes;
    invalid_timestamp[position + 2..position + 10].copy_from_slice(&i64::MAX.to_be_bytes());
    rejected(&invalid_timestamp);
}
