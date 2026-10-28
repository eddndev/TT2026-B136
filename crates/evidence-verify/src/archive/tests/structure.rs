use super::super::admit;
use super::writer::{central, end, local, write, Built};
use super::{assert_not_supported, sample};

fn shift_offsets(built: &mut Built, delta: u32) {
    let directory_offset = built.get_u32(built.end + end::DIRECTORY_OFFSET);
    built.set_u32(built.end + end::DIRECTORY_OFFSET, directory_offset + delta);
    for index in 0..built.centrals.len() {
        let at = built.centrals[index] + central::LOCAL_OFFSET;
        let offset = built.get_u32(at);
        built.set_u32(at, offset + delta);
    }
}

#[test]
fn inputs_without_an_end_record_are_not_supported() {
    assert_not_supported(b"", "zip.end_record");
    assert_not_supported(&[0u8; 21], "zip.end_record");
    assert_not_supported(&[0u8; 400], "zip.end_record");
    assert_not_supported(b"PK\x05\x06", "zip.end_record");
}

#[test]
fn trailing_bytes_after_the_end_record_are_not_supported() {
    let mut built = sample();
    built.bytes.push(0);
    assert_not_supported(&built.bytes, "zip.end_record");
}

#[test]
fn an_archive_comment_is_not_supported() {
    let mut built = sample();
    built.set_u16(built.end + end::COMMENT_LEN, 3);
    built.bytes.extend_from_slice(b"abc");
    assert_not_supported(&built.bytes, "zip.comment");

    let mut built = sample();
    built.set_u16(built.end + end::COMMENT_LEN, 3);
    assert_not_supported(&built.bytes, "zip.comment");
}

#[test]
fn a_second_end_record_is_not_supported() {
    let mut built = sample();
    let record = built.bytes[built.end..].to_vec();
    built.bytes.extend_from_slice(&record);
    assert_not_supported(&built.bytes, "zip.layout");
}

#[test]
fn multi_disk_archives_are_not_supported() {
    let mut built = sample();
    built.set_u16(built.end + end::DISK, 1);
    assert_not_supported(&built.bytes, "zip.multi_disk");

    let mut built = sample();
    built.set_u16(built.end + end::DIRECTORY_DISK, 1);
    assert_not_supported(&built.bytes, "zip.multi_disk");
}

#[test]
fn zip64_sentinels_in_the_end_record_are_not_supported() {
    let mut built = sample();
    built.set_u16(built.end + end::ENTRIES_ON_DISK, u16::MAX);
    built.set_u16(built.end + end::ENTRIES, u16::MAX);
    assert_not_supported(&built.bytes, "zip.zip64");

    let mut built = sample();
    built.set_u32(built.end + end::DIRECTORY_SIZE, u32::MAX);
    assert_not_supported(&built.bytes, "zip.zip64");

    let mut built = sample();
    built.set_u32(built.end + end::DIRECTORY_OFFSET, u32::MAX);
    assert_not_supported(&built.bytes, "zip.zip64");
}

#[test]
fn an_archive_without_entries_is_not_supported() {
    let mut bytes = b"PK\x05\x06".to_vec();
    bytes.extend_from_slice(&[0u8; 18]);
    assert_not_supported(&bytes, "zip.empty");
}

#[test]
fn entry_counts_that_disagree_are_not_supported() {
    let mut built = sample();
    built.set_u16(built.end + end::ENTRIES_ON_DISK, 6);
    assert_not_supported(&built.bytes, "zip.entry_count");

    for count in [6, 8] {
        let mut built = sample();
        built.set_u16(built.end + end::ENTRIES_ON_DISK, count);
        built.set_u16(built.end + end::ENTRIES, count);
        assert_not_supported(&built.bytes, "zip.entry_count");
    }
}

#[test]
fn a_directory_that_does_not_end_at_the_end_record_is_not_supported() {
    let mut built = sample();
    let size = built.get_u32(built.end + end::DIRECTORY_SIZE);
    built.set_u32(built.end + end::DIRECTORY_SIZE, size - 1);
    assert_not_supported(&built.bytes, "zip.layout");

    let mut built = sample();
    let offset = built.get_u32(built.end + end::DIRECTORY_OFFSET);
    built.set_u32(built.end + end::DIRECTORY_OFFSET, offset + 1);
    assert_not_supported(&built.bytes, "zip.layout");
}

#[test]
fn bytes_before_the_first_entry_are_not_supported() {
    let mut built = sample();
    built.bytes.insert(0, 0);
    built.end += 1;
    for index in 0..built.centrals.len() {
        built.centrals[index] += 1;
    }
    shift_offsets(&mut built, 1);
    assert_not_supported(&built.bytes, "zip.local_header");
}

#[test]
fn a_gap_before_the_directory_is_not_supported() {
    let mut built = sample();
    let directory_offset = built.get_u32(built.end + end::DIRECTORY_OFFSET) as usize;
    built.bytes.insert(directory_offset, 0);
    built.end += 1;
    let at = built.end + end::DIRECTORY_OFFSET;
    built.set_u32(at, directory_offset as u32 + 1);
    assert_not_supported(&built.bytes, "zip.layout");
}

#[test]
fn an_entry_that_overruns_the_directory_is_not_supported() {
    let mut built = sample();
    let last = built.locals[6];
    let size = built.get_u32(last + local::SIZE);
    built.set_u32(last + local::SIZE, size + 1);
    built.set_u32(last + local::COMPRESSED_SIZE, size + 1);
    assert_not_supported(&built.bytes, "zip.layout");
}

#[test]
fn a_wrong_local_signature_is_not_supported() {
    let mut built = sample();
    built.bytes[built.locals[3] + local::SIGNATURE + 3] = 0x05;
    assert_not_supported(&built.bytes, "zip.local_header");
}

#[test]
fn a_wrong_central_signature_is_not_supported() {
    let mut built = sample();
    built.bytes[built.centrals[3] + central::SIGNATURE + 3] = 0x05;
    assert_not_supported(&built.bytes, "zip.central_header");
}

#[test]
fn a_central_header_pointing_elsewhere_is_not_supported() {
    let mut built = sample();
    let first = built.get_u32(built.centrals[0] + central::LOCAL_OFFSET);
    built.set_u32(built.centrals[1] + central::LOCAL_OFFSET, first);
    assert_not_supported(&built.bytes, "zip.header_mismatch");
}

#[test]
fn a_reordered_directory_is_not_supported() {
    let mut built = write(&[("a.bin", b"1"), ("b.bin", b"2")]);
    let first = built.centrals[0];
    let second = built.centrals[1];
    let len = second - first;
    let one = built.bytes[first..second].to_vec();
    let two = built.bytes[second..second + len].to_vec();
    built.bytes[first..second].copy_from_slice(&two);
    built.bytes[second..second + len].copy_from_slice(&one);
    assert_not_supported(&built.bytes, "zip.header_mismatch");
}

#[test]
fn names_outside_the_safe_set_are_not_supported() {
    let long = "a".repeat(129);
    for name in [
        "",
        ".hidden",
        "-flag",
        "_x",
        "a/b.txt",
        "a\\b.txt",
        "../x",
        "con espacio",
        "acta\u{00f3}.pdf",
        "a\0b",
        long.as_str(),
    ] {
        let built = write(&[(name, b"x")]);
        assert_not_supported(&built.bytes, "zip.entry_name");
    }
}

#[test]
fn names_inside_the_safe_set_are_admitted() {
    let long = "Z".repeat(128);
    for name in ["0", "a", "doc-1.txt.sig", "ca_root.pem", long.as_str()] {
        let archive = admit(&write(&[(name, b"x")]).bytes).unwrap();
        assert_eq!(archive.names(), [name]);
    }
}

#[test]
fn duplicate_names_are_not_supported() {
    let built = write(&[("a.txt", b"1"), ("b.txt", b"2"), ("a.txt", b"3")]);
    assert_not_supported(&built.bytes, "zip.duplicate_name");
}

#[test]
fn names_differing_only_in_case_are_distinct() {
    let archive = admit(&write(&[("a.txt", b"1"), ("A.txt", b"2")]).bytes).unwrap();
    assert_eq!(archive.get("a.txt"), Some(&b"1"[..]));
    assert_eq!(archive.get("A.txt"), Some(&b"2"[..]));
}

#[test]
fn structural_defects_take_precedence_over_a_crc_mismatch() {
    let mut built = sample();
    let crc = built.get_u32(built.locals[0] + local::CRC) ^ 1;
    built.set_u32(built.locals[0] + local::CRC, crc);
    built.set_u32(built.centrals[0] + central::CRC, crc);
    built.set_u16(built.centrals[6] + central::INTERNAL_ATTRIBUTES, 1);
    assert_not_supported(&built.bytes, "zip.attributes");
}
