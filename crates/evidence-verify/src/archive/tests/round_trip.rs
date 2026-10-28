use super::super::crc::crc32;
use super::super::{admit, Archive, Entry};
use super::{refusal, sample, writer};
use crate::report::Outcome;

#[test]
fn crc32_matches_the_standard_check_value() {
    assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    assert_eq!(crc32(b""), 0);
    assert_eq!(crc32(b"x"), 0x8cdc_1683);
}

#[test]
fn crc32_matches_the_bitwise_definition_on_every_byte_value() {
    let data: Vec<u8> = (0..=255u8).cycle().take(1000).collect();
    let mut crc = u32::MAX;
    for byte in &data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    assert_eq!(crc32(&data), !crc);
}

#[test]
fn the_test_writer_reproduces_the_server_layout_byte_for_byte() {
    let expected: Vec<u8> = [
        // Local header, name "a", content "x".
        &b"PK\x03\x04"[..],
        &[10, 0, 0, 0, 0, 0, 0, 0, 0x21, 0],
        &[0x83, 0x16, 0xdc, 0x8c, 1, 0, 0, 0, 1, 0, 0, 0],
        &[1, 0, 0, 0],
        b"a",
        b"x",
        // Central directory header.
        b"PK\x01\x02",
        &[20, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0x21, 0],
        &[0x83, 0x16, 0xdc, 0x8c, 1, 0, 0, 0, 1, 0, 0, 0],
        &[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        b"a",
        // End record: one entry, directory of 47 bytes at offset 32.
        b"PK\x05\x06",
        &[0, 0, 0, 0, 1, 0, 1, 0, 47, 0, 0, 0, 32, 0, 0, 0, 0, 0],
    ]
    .concat();
    assert_eq!(writer::write(&[("a", b"x")]).bytes, expected);
}

#[test]
fn a_package_round_trips_in_entry_order() {
    let built = sample();
    let archive = admit(&built.bytes).unwrap();
    assert_eq!(
        archive.names(),
        [
            "acta.pdf",
            "acta.pdf.sig",
            "acta.pdf.tsr",
            "certificado.pem",
            "ca.pem",
            "crl.pem",
            "INSTRUCCIONES.md",
        ]
    );
    assert_eq!(archive.get("acta.pdf"), Some(&b"%PDF-1.7 body"[..]));
    assert_eq!(archive.get("crl.pem"), Some(&b""[..]));
    assert_eq!(archive.get("missing.pem"), None);
    assert_eq!(
        archive.entries[4],
        Entry {
            name: "ca.pem".to_string(),
            data: b"ca".to_vec(),
        }
    );
}

#[test]
fn names_are_looked_up_byte_for_byte() {
    let archive = admit(&sample().bytes).unwrap();
    assert_eq!(archive.get("CA.pem"), None);
    assert_eq!(archive.get("instrucciones.md"), None);
}

#[test]
fn a_single_empty_entry_is_admitted() {
    let archive = admit(&writer::write(&[("empty.bin", b"")]).bytes).unwrap();
    assert_eq!(
        archive,
        Archive {
            entries: vec![Entry {
                name: "empty.bin".to_string(),
                data: Vec::new(),
            }],
        }
    );
}

#[test]
fn binary_content_resembling_records_is_kept_verbatim() {
    let content = b"PK\x05\x06PK\x01\x02PK\x03\x04\x00\xff";
    let archive = admit(&writer::write(&[("x.bin", content)]).bytes).unwrap();
    assert_eq!(archive.get("x.bin"), Some(&content[..]));
}

#[test]
fn a_crc_mismatch_in_both_headers_fails_the_package() {
    let mut built = sample();
    let crc = built.get_u32(built.locals[2] + writer::local::CRC) ^ 1;
    built.set_u32(built.locals[2] + writer::local::CRC, crc);
    built.set_u32(built.centrals[2] + writer::central::CRC, crc);
    let finding = refusal(&built.bytes);
    assert_eq!(finding.outcome, Outcome::Failed);
    assert_eq!(finding.code, "zip.crc_mismatch");
    assert!(finding.detail.contains("acta.pdf.tsr"), "{finding:?}");
}

#[test]
fn altered_content_fails_the_package() {
    let mut built = sample();
    let at = built.locals[0] + writer::local::NAME + "acta.pdf".len();
    built.bytes[at] ^= 0x20;
    let finding = refusal(&built.bytes);
    assert_eq!(finding.outcome, Outcome::Failed);
    assert_eq!(finding.code, "zip.crc_mismatch");
}
