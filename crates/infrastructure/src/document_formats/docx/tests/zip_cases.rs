use super::{
    fixtures::{self, document, WORD},
    rejected,
};

#[test]
fn central_and_local_headers_must_match_without_prefix_split_or_zip64() {
    let original = fixtures::normal(&document(WORD, ""), &[]);
    for field in 0..5 {
        let mut bytes = original.clone();
        let eocd = fixtures::signature(&bytes, b"PK\x05\x06");
        let central = fixtures::signature(&bytes, b"PK\x01\x02");
        match field {
            0 => bytes[6] ^= 8,
            1 => bytes[eocd + 4] = 1,
            2 => bytes[eocd + 10..eocd + 12].copy_from_slice(&u16::MAX.to_le_bytes()),
            3 => bytes[central + 46] = 0xff,
            _ => bytes.splice(0..0, [0, 1, 2]).for_each(drop),
        }
        rejected(&bytes);
    }
}

#[test]
fn signed_and_unsigned_descriptors_are_accepted_only_when_consistent() {
    let parts = super::raw_zip::parts(&fixtures::normal(&document(WORD, ""), &[]));
    for signed in [false, true] {
        let mut bytes = super::raw_zip::archive(&parts, Some(signed), false);
        super::accepted(&bytes);
        let end = fixtures::signature(&bytes, b"PK\x03\x04");
        assert_eq!(end, 0);
        let next = bytes[4..]
            .windows(4)
            .position(|w| w == b"PK\x03\x04")
            .unwrap()
            + 4;
        bytes[next - 1] ^= 1;
        rejected(&bytes);
    }
}

#[test]
fn raw_duplicate_and_case_equivalent_names_are_rejected_before_archive_indexing() {
    for same_spelling in [true, false] {
        let mut parts = super::raw_zip::parts(&fixtures::normal(&document(WORD, ""), &[]));
        parts.push(fixtures::Part {
            name: if same_spelling {
                "word/document.xml".into()
            } else {
                "WORD/DOCUMENT.XML".into()
            },
            bytes: document(WORD, "").into_bytes(),
        });
        rejected(&super::raw_zip::archive(&parts, None, false));
    }
}

#[test]
fn trailing_bytes_inside_the_declared_deflate_payload_are_rejected() {
    let parts = super::raw_zip::parts(&fixtures::normal(&document(WORD, ""), &[]));
    rejected(&super::raw_zip::archive(&parts, None, true));
}

#[test]
fn stored_entries_are_supported_and_local_dates_must_match_the_directory() {
    use std::io::{Cursor, Write};
    use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};
    let parts = super::raw_zip::parts(&fixtures::normal(&document(WORD, ""), &[]));
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for part in parts {
        writer
            .start_file(
                part.name,
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
            )
            .unwrap();
        writer.write_all(&part.bytes).unwrap();
    }
    let mut bytes = writer.finish().unwrap().into_inner();
    super::accepted(&bytes);
    bytes[12] ^= 1;
    rejected(&bytes);
}

#[test]
fn package_uris_require_valid_escapes_without_unencoded_brackets_or_traversal() {
    for name in [
        "other[1].xml",
        "other%FF.xml",
        "../other.xml",
        "other%2Fpart.xml",
        "other%61.xml",
    ] {
        rejected(&fixtures::normal(
            &document(WORD, ""),
            &[fixtures::Part::text(name, "<other/>")],
        ));
    }
    super::accepted(&fixtures::normal(
        &document(WORD, ""),
        &[fixtures::Part::text("word/%CE%B1.xml", "<other/>")],
    ));
}

#[test]
fn a_symlink_entry_cannot_be_treated_as_an_ordinary_part() {
    let mut bytes = fixtures::normal(&document(WORD, ""), &[]);
    let central = fixtures::signature(&bytes, b"PK\x01\x02");
    bytes[central + 5] = 3;
    bytes[central + 38..central + 42].copy_from_slice(&(0o120777u32 << 16).to_le_bytes());
    rejected(&bytes);
}
