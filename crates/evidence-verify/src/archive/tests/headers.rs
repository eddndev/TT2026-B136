use super::writer::{central, local, Built};
use super::{assert_not_supported, sample};

/// A field patch inside the header of the third entry.
enum Patch {
    Local16(usize, u16),
    Local32(usize, u32),
    Central16(usize, u16),
    Central32(usize, u32),
}

fn patched(patches: &[Patch]) -> Built {
    let mut built = sample();
    let (at_local, at_central) = (built.locals[2], built.centrals[2]);
    for patch in patches {
        match *patch {
            Patch::Local16(field, value) => built.set_u16(at_local + field, value),
            Patch::Local32(field, value) => built.set_u32(at_local + field, value),
            Patch::Central16(field, value) => built.set_u16(at_central + field, value),
            Patch::Central32(field, value) => built.set_u32(at_central + field, value),
        }
    }
    built
}

#[test]
fn local_header_fields_outside_the_writer_profile_are_not_supported() {
    let cases = [
        (Patch::Local16(local::VERSION_NEEDED, 20), "zip.version"),
        (Patch::Local16(local::VERSION_NEEDED, 0), "zip.version"),
        (Patch::Local16(local::FLAGS, 0x0001), "zip.encrypted"),
        (Patch::Local16(local::FLAGS, 0x0041), "zip.encrypted"),
        (Patch::Local16(local::FLAGS, 0x0008), "zip.data_descriptor"),
        (Patch::Local16(local::FLAGS, 0x0800), "zip.flags"),
        (Patch::Local16(local::METHOD, 8), "zip.method"),
        (Patch::Local16(local::TIME, 1), "zip.timestamp"),
        (Patch::Local16(local::DATE, 0x0022), "zip.timestamp"),
        (Patch::Local16(local::EXTRA_LEN, 4), "zip.extra_field"),
        (Patch::Local32(local::COMPRESSED_SIZE, 4), "zip.stored_size"),
        (Patch::Local32(local::SIZE, u32::MAX), "zip.zip64"),
        (
            Patch::Local32(local::COMPRESSED_SIZE, u32::MAX),
            "zip.zip64",
        ),
    ];
    for (patch, code) in cases {
        assert_not_supported(&patched(&[patch]).bytes, code);
    }
}

#[test]
fn central_header_fields_outside_the_writer_profile_are_not_supported() {
    let cases = [
        (
            Patch::Central16(central::VERSION_MADE_BY, 0x0314),
            "zip.version",
        ),
        (
            Patch::Central16(central::VERSION_MADE_BY, 10),
            "zip.version",
        ),
        (Patch::Central16(central::VERSION_NEEDED, 20), "zip.version"),
        (Patch::Central16(central::FLAGS, 0x0001), "zip.encrypted"),
        (
            Patch::Central16(central::FLAGS, 0x0008),
            "zip.data_descriptor",
        ),
        (Patch::Central16(central::FLAGS, 0x0800), "zip.flags"),
        (Patch::Central16(central::METHOD, 8), "zip.method"),
        (Patch::Central16(central::TIME, 1), "zip.timestamp"),
        (Patch::Central16(central::DATE, 0), "zip.timestamp"),
        (Patch::Central16(central::EXTRA_LEN, 4), "zip.extra_field"),
        (Patch::Central16(central::COMMENT_LEN, 1), "zip.comment"),
        (Patch::Central16(central::DISK_START, 1), "zip.multi_disk"),
        (
            Patch::Central16(central::INTERNAL_ATTRIBUTES, 1),
            "zip.attributes",
        ),
        (
            Patch::Central32(central::EXTERNAL_ATTRIBUTES, 0x81a4_0000),
            "zip.attributes",
        ),
        (
            Patch::Central32(central::EXTERNAL_ATTRIBUTES, 0x20),
            "zip.attributes",
        ),
        (
            Patch::Central32(central::COMPRESSED_SIZE, 4),
            "zip.stored_size",
        ),
        (Patch::Central32(central::SIZE, u32::MAX), "zip.zip64"),
        (
            Patch::Central32(central::LOCAL_OFFSET, u32::MAX),
            "zip.zip64",
        ),
    ];
    for (patch, code) in cases {
        assert_not_supported(&patched(&[patch]).bytes, code);
    }
}

#[test]
fn the_same_foreign_field_in_both_headers_is_still_not_supported() {
    let cases = [
        (local::METHOD, central::METHOD, 8, "zip.method"),
        (local::FLAGS, central::FLAGS, 0x0008, "zip.data_descriptor"),
        (local::DATE, central::DATE, 0x5821, "zip.timestamp"),
        (
            local::VERSION_NEEDED,
            central::VERSION_NEEDED,
            45,
            "zip.version",
        ),
    ];
    for (at_local, at_central, value, code) in cases {
        let built = patched(&[
            Patch::Local16(at_local, value),
            Patch::Central16(at_central, value),
        ]);
        assert_not_supported(&built.bytes, code);
    }
}

#[test]
fn headers_that_disagree_are_not_supported() {
    let built = sample();
    let crc = built.get_u32(built.locals[2] + local::CRC);
    let size = built.get_u32(built.locals[2] + local::SIZE);
    let cases = [
        vec![Patch::Central32(central::CRC, crc ^ 1)],
        vec![Patch::Local32(local::CRC, crc ^ 1)],
        vec![
            Patch::Central32(central::SIZE, size - 1),
            Patch::Central32(central::COMPRESSED_SIZE, size - 1),
        ],
    ];
    for patches in cases {
        assert_not_supported(&patched(&patches).bytes, "zip.header_mismatch");
    }
}

#[test]
fn a_central_name_that_differs_from_the_local_name_is_not_supported() {
    let mut built = sample();
    let at = built.centrals[2] + central::NAME;
    built.bytes[at] = b'b';
    assert_not_supported(&built.bytes, "zip.header_mismatch");

    let mut built = sample();
    let at = built.centrals[4] + central::NAME;
    built.bytes[at] = b'C';
    assert_not_supported(&built.bytes, "zip.header_mismatch");
}

#[test]
fn a_central_name_outside_the_safe_set_is_not_supported() {
    let mut built = sample();
    let at = built.centrals[2] + central::NAME;
    built.bytes[at] = b'/';
    assert_not_supported(&built.bytes, "zip.entry_name");
}
