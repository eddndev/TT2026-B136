use super::*;

fn tlv_bytes(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    let len = value.len();
    if len < 0x80 {
        out.push(len as u8);
    } else {
        let bytes = (len as u32).to_be_bytes();
        let skip = bytes.iter().take_while(|b| **b == 0).count();
        out.push(0x80 | (4 - skip) as u8);
        out.extend_from_slice(&bytes[skip..]);
    }
    out.extend_from_slice(value);
    out
}

fn nested_sequences(depth: usize) -> Vec<u8> {
    let mut inner = vec![0x05, 0x00];
    for _ in 1..depth {
        inner = tlv_bytes(0x30, &inner);
    }
    inner
}

#[test]
fn reads_short_form_and_returns_rest() {
    let input = [0x02, 0x01, 0x05, 0xAA, 0xBB];
    let (tlv, rest) = read(&input).unwrap();
    assert_eq!(tlv.tag, 0x02);
    assert_eq!(tlv.header, &[0x02, 0x01]);
    assert_eq!(tlv.value, &[0x05]);
    assert_eq!(tlv.raw, &[0x02, 0x01, 0x05]);
    assert_eq!(rest, &[0xAA, 0xBB]);
}

#[test]
fn raw_is_the_original_slice_not_a_copy() {
    let input = tlv_bytes(0x30, &tlv_bytes(0x04, b"abc"));
    let (tlv, rest) = read(&input).unwrap();
    assert!(rest.is_empty());
    assert_eq!(tlv.raw.as_ptr(), input.as_ptr());
    assert_eq!(tlv.header.as_ptr(), input.as_ptr());
    assert_eq!(tlv.value.as_ptr(), input[tlv.header.len()..].as_ptr());
    assert_eq!(tlv.header.len() + tlv.value.len(), tlv.raw.len());
}

#[test]
fn reads_empty_value() {
    let (tlv, rest) = read(&[0x05, 0x00]).unwrap();
    assert_eq!(tlv.tag, 0x05);
    assert!(tlv.value.is_empty());
    assert_eq!(tlv.raw.len(), 2);
    assert!(rest.is_empty());
}

#[test]
fn reads_largest_short_form() {
    let input = tlv_bytes(0x04, &[0x11; 0x7F]);
    assert_eq!(input[1], 0x7F);
    let tlv = read_exact(&input).unwrap();
    assert_eq!(tlv.header.len(), 2);
    assert_eq!(tlv.value.len(), 0x7F);
}

#[test]
fn reads_one_byte_long_form() {
    let input = tlv_bytes(0x04, &[0x22; 0x80]);
    assert_eq!(&input[..3], &[0x04, 0x81, 0x80]);
    let tlv = read_exact(&input).unwrap();
    assert_eq!(tlv.header.len(), 3);
    assert_eq!(tlv.value.len(), 0x80);
}

#[test]
fn reads_two_byte_long_form() {
    let input = tlv_bytes(0x04, &[0x33; 0x100]);
    assert_eq!(&input[..4], &[0x04, 0x82, 0x01, 0x00]);
    let tlv = read_exact(&input).unwrap();
    assert_eq!(tlv.header.len(), 4);
    assert_eq!(tlv.value.len(), 0x100);
}

#[test]
fn reads_three_byte_long_form() {
    let input = tlv_bytes(0x04, &vec![0x44; 0x1_0000]);
    assert_eq!(&input[..5], &[0x04, 0x83, 0x01, 0x00, 0x00]);
    let tlv = read_exact(&input).unwrap();
    assert_eq!(tlv.header.len(), 5);
    assert_eq!(tlv.value.len(), 0x1_0000);
}

#[test]
fn four_byte_long_form_beyond_input_is_truncated() {
    assert_eq!(
        read(&[0x04, 0x84, 0x01, 0x00, 0x00, 0x00, 0x00]),
        Err(DerError::Truncated)
    );
    assert_eq!(
        read(&[0x04, 0x84, 0xFF, 0xFF, 0xFF, 0xFF]),
        Err(DerError::Truncated)
    );
}

#[test]
fn rejects_empty_and_tag_only_input() {
    assert_eq!(read(&[]), Err(DerError::Truncated));
    assert_eq!(read(&[0x30]), Err(DerError::Truncated));
}

#[test]
fn rejects_missing_length_bytes() {
    assert_eq!(read(&[0x04, 0x81]), Err(DerError::Truncated));
    assert_eq!(read(&[0x04, 0x82, 0x01]), Err(DerError::Truncated));
    assert_eq!(read(&[0x04, 0x83, 0x01, 0x00]), Err(DerError::Truncated));
    assert_eq!(
        read(&[0x04, 0x84, 0x01, 0x00, 0x00]),
        Err(DerError::Truncated)
    );
}

#[test]
fn rejects_value_shorter_than_declared() {
    assert_eq!(read(&[0x04, 0x03, 0x01, 0x02]), Err(DerError::Truncated));
    let mut input = vec![0x04, 0x81, 0x80];
    input.extend_from_slice(&[0; 0x7F]);
    assert_eq!(read(&input), Err(DerError::Truncated));
}

#[test]
fn rejects_indefinite_length() {
    assert_eq!(
        read(&[0x30, 0x80, 0x05, 0x00, 0x00, 0x00]),
        Err(DerError::IndefiniteLength)
    );
    assert_eq!(read(&[0x30, 0x80]), Err(DerError::IndefiniteLength));
}

#[test]
fn rejects_long_form_for_short_lengths() {
    let mut input = vec![0x04, 0x81, 0x7F];
    input.extend_from_slice(&[0; 0x7F]);
    assert_eq!(read(&input), Err(DerError::NonMinimalLength));
    assert_eq!(read(&[0x04, 0x81, 0x00]), Err(DerError::NonMinimalLength));
}

#[test]
fn rejects_leading_zero_length_bytes() {
    let mut input = vec![0x04, 0x82, 0x00, 0x80];
    input.extend_from_slice(&[0; 0x80]);
    assert_eq!(read(&input), Err(DerError::NonMinimalLength));
    assert_eq!(
        read(&[0x04, 0x83, 0x00, 0x01, 0x00]),
        Err(DerError::NonMinimalLength)
    );
    assert_eq!(
        read(&[0x04, 0x84, 0x00, 0x00, 0x00, 0x00]),
        Err(DerError::NonMinimalLength)
    );
}

#[test]
fn rejects_more_than_four_length_bytes() {
    assert_eq!(
        read(&[0x04, 0x85, 0x01, 0x00, 0x00, 0x00, 0x00]),
        Err(DerError::Truncated)
    );
    assert_eq!(
        read(&[0x04, 0x85, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00]),
        Err(DerError::Truncated)
    );
    assert_eq!(read(&[0x04, 0xFF]), Err(DerError::Truncated));
}

#[test]
fn rejects_high_tag_numbers() {
    for tag in [0x1F, 0x3F, 0x5F, 0x9F, 0xBF, 0xFF] {
        assert_eq!(read(&[tag, 0x1F, 0x00]), Err(DerError::HighTagNumber));
        assert_eq!(read(&[tag]), Err(DerError::HighTagNumber));
    }
}

#[test]
fn accepts_largest_low_tag_numbers() {
    for tag in [0x1E, 0x3E, 0x9E, 0xBE] {
        let input = [tag, 0x00];
        let tlv = read_exact(&input).unwrap();
        assert_eq!(tlv.tag, tag);
    }
}

#[test]
fn constructed_bit_is_reported() {
    assert!(read_exact(&[0x30, 0x00]).unwrap().is_constructed());
    assert!(read_exact(&[0x31, 0x00]).unwrap().is_constructed());
    assert!(read_exact(&[0xA0, 0x00]).unwrap().is_constructed());
    assert!(!read_exact(&[0x80, 0x00]).unwrap().is_constructed());
    assert!(!read_exact(&[0x04, 0x00]).unwrap().is_constructed());
}

#[test]
fn read_exact_rejects_trailing_bytes() {
    assert_eq!(read_exact(&[0x05, 0x00, 0x00]), Err(DerError::TrailingData));
    assert_eq!(
        read_exact(&[0x05, 0x00, 0x05, 0x00]),
        Err(DerError::TrailingData)
    );
}

#[test]
fn read_exact_propagates_read_errors() {
    assert_eq!(read_exact(&[]), Err(DerError::Truncated));
    assert_eq!(read_exact(&[0x30, 0x80]), Err(DerError::IndefiniteLength));
}

#[test]
fn children_splits_a_sequence_value() {
    let mut value = tlv_bytes(0x02, &[0x01]);
    value.extend(tlv_bytes(0xA0, &tlv_bytes(0x04, b"xy")));
    value.extend(tlv_bytes(0x05, &[]));
    let outer = tlv_bytes(0x30, &value);
    let seq = read_exact(&outer).unwrap();
    let kids = children(seq.value, 8).unwrap();
    assert_eq!(kids.len(), 3);
    assert_eq!(kids[0].tag, 0x02);
    assert_eq!(kids[0].value, &[0x01]);
    assert_eq!(kids[1].tag, 0xA0);
    assert_eq!(kids[1].raw, &[0xA0, 0x04, 0x04, 0x02, b'x', b'y']);
    assert_eq!(kids[2].tag, 0x05);
    let offset = kids[1].raw.as_ptr() as usize - outer.as_ptr() as usize;
    assert_eq!(offset, 2 + 3);
    let grand = children(kids[1].value, 1).unwrap();
    assert_eq!(grand.len(), 1);
    assert_eq!(grand[0].value, b"xy");
}

#[test]
fn children_of_empty_value_is_empty() {
    assert!(children(&[], 0).unwrap().is_empty());
    assert!(children(&[], 4).unwrap().is_empty());
}

#[test]
fn children_limit_is_inclusive() {
    let value = [0x05, 0x00, 0x05, 0x00, 0x05, 0x00];
    assert_eq!(children(&value, 3).unwrap().len(), 3);
    assert_eq!(children(&value, 2), Err(DerError::TooManyChildren));
    assert_eq!(children(&value, 0), Err(DerError::TooManyChildren));
}

#[test]
fn children_rejects_truncated_child() {
    let value = [0x05, 0x00, 0x04, 0x05, 0x01];
    assert_eq!(children(&value, 8), Err(DerError::Truncated));
    let outer = [0x30, 0x05, 0x05, 0x00, 0x04, 0x05, 0x01];
    let seq = read_exact(&outer).unwrap();
    assert_eq!(children(seq.value, 8), Err(DerError::Truncated));
}

#[test]
fn children_rejects_malformed_child() {
    assert_eq!(
        children(&[0x05, 0x00, 0x30, 0x80], 8),
        Err(DerError::IndefiniteLength)
    );
    assert_eq!(
        children(&[0x1F, 0x01, 0x00], 8),
        Err(DerError::HighTagNumber)
    );
    assert_eq!(
        children(&[0x04, 0x81, 0x01, 0x00], 8),
        Err(DerError::NonMinimalLength)
    );
}

#[test]
fn walk_accepts_nesting_up_to_the_limit() {
    let input = nested_sequences(MAX_DEPTH);
    let tlv = walk(&input, MAX_DEPTH).unwrap();
    assert_eq!(tlv.raw.as_ptr(), input.as_ptr());
    assert_eq!(tlv.raw.len(), input.len());
}

#[test]
fn walk_rejects_nesting_beyond_the_limit() {
    let input = nested_sequences(MAX_DEPTH + 1);
    assert_eq!(walk(&input, MAX_DEPTH), Err(DerError::TooDeep));
    assert!(walk(&input, MAX_DEPTH + 1).is_ok());
}

#[test]
fn walk_counts_the_outer_element_as_depth_one() {
    assert!(walk(&[0x05, 0x00], 1).is_ok());
    assert!(walk(&[0x30, 0x00], 1).is_ok());
    assert_eq!(walk(&[0x30, 0x02, 0x05, 0x00], 1), Err(DerError::TooDeep));
    assert!(walk(&[0x30, 0x02, 0x05, 0x00], 2).is_ok());
}

#[test]
fn walk_with_zero_depth_rejects_everything() {
    assert_eq!(walk(&[0x05, 0x00], 0), Err(DerError::TooDeep));
}

#[test]
fn walk_does_not_descend_into_primitive_values() {
    let not_der = [0x30, 0x80, 0xFF, 0x1F];
    let octets = tlv_bytes(0x04, &not_der);
    let explicit = tlv_bytes(0xA0, &octets);
    let input = tlv_bytes(0x30, &explicit);
    assert!(walk(&input, 3).is_ok());
    let implicit = tlv_bytes(0x80, &not_der);
    assert!(walk(&tlv_bytes(0x30, &implicit), 2).is_ok());
}

#[test]
fn walk_descends_into_context_constructed_values() {
    let bad_inner = tlv_bytes(0xA0, &[0x30, 0x80]);
    assert_eq!(
        walk(&tlv_bytes(0x30, &bad_inner), MAX_DEPTH),
        Err(DerError::IndefiniteLength)
    );
}

#[test]
fn walk_rejects_truncated_grandchild() {
    let broken = [0x30, 0x04, 0x30, 0x02, 0x04, 0x05];
    assert_eq!(walk(&broken, MAX_DEPTH), Err(DerError::Truncated));
}

#[test]
fn walk_rejects_trailing_data_inside_and_outside() {
    assert_eq!(
        walk(&[0x05, 0x00, 0x00], MAX_DEPTH),
        Err(DerError::TrailingData)
    );
    let inner_trailing = [0x30, 0x03, 0x05, 0x00, 0x00];
    assert_eq!(walk(&inner_trailing, MAX_DEPTH), Err(DerError::Truncated));
}

#[test]
fn walk_visits_every_sibling() {
    let mut value = tlv_bytes(0x30, &[0x05, 0x00]);
    value.extend(tlv_bytes(0x31, &[0x1F, 0x00, 0x00]));
    assert_eq!(
        walk(&tlv_bytes(0x30, &value), MAX_DEPTH),
        Err(DerError::HighTagNumber)
    );
}

#[test]
fn errors_have_stable_codes_and_ascii_messages() {
    let all = [
        (DerError::Truncated, "der.truncated"),
        (DerError::IndefiniteLength, "der.indefinite_length"),
        (DerError::NonMinimalLength, "der.non_minimal_length"),
        (DerError::HighTagNumber, "der.high_tag_number"),
        (DerError::TooManyChildren, "der.too_many_children"),
        (DerError::TrailingData, "der.trailing_data"),
        (DerError::TooDeep, "der.too_deep"),
    ];
    for (error, code) in all {
        assert_eq!(error.code(), code);
        let message = error.to_string();
        assert!(!message.is_empty());
        assert!(message.is_ascii());
    }
}
