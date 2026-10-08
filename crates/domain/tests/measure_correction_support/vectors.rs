use domain::precautionary_measures::{MeasureCorrectionValues, MeasureValidity};
use domain::procedural_time::DeclaredProceduralTime as Declared;
use sha2::{Digest, Sha256};
use time::UtcOffset;

use crate::support::*;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn assert_vector(value: MeasureCorrectionValues, length: usize, bytes: &str, digest: &str) {
    let encoded = value.canonical_bytes();
    assert_eq!(encoded.len(), length);
    assert_eq!(hex(&encoded), bytes);
    assert_eq!(hex(&Sha256::digest(&encoded)), digest);
}

#[test]
fn date_only_correction_matches_independent_mcval1_bytes_and_sha256() {
    let value = MeasureCorrectionValues::new(
        note("C\u{e9}"),
        MeasureValidity::new(known(Declared::date(day(), None).unwrap()), note("V"), None).unwrap(),
        note("S"),
    );
    // Literal frame: MCVAL1, UTF-8 conditions, full MVAL1 blob, supervision text.
    let expected = concat!(
        "4d4356414c31",   // MCVAL1
        "0000000343c3a9", // Three UTF-8 bytes
        "00000012",       // Full nested validity length
        "4d56414c31",     // MVAL1
        "0107ea0a040000", // Date, absent offset, absent unknown reason
        "0000000156",     // Validity statement
        "00",             // No end
        "0000000153"      // Supervision text
    );
    assert_vector(
        value,
        40,
        expected,
        "35be1cd45dd2d26ec387277a1e64293c8dc06a9e1245023a94dea0221d86cc58",
    );
}

#[test]
fn mixed_precision_end_matches_independent_mcval1_bytes_and_sha256() {
    let value = MeasureCorrectionValues::new(
        note("C\u{e9}"),
        MeasureValidity::new(
            known(
                Declared::second(day(), 1, 2, 3, Some(UtcOffset::from_hms(-6, 0, 0).unwrap()))
                    .unwrap(),
            ),
            note("V"),
            Some(known(
                Declared::minute(
                    "2026-10-05".parse().unwrap(),
                    4,
                    5,
                    Some(UtcOffset::from_hms(5, 45, 0).unwrap()),
                )
                .unwrap(),
            )),
        )
        .unwrap(),
        note("S\u{f1}"),
    );
    let expected = concat!(
        "4d4356414c31",
        "0000000343c3a9",
        "00000026",
        "4d56414c31",
        "0307ea0a0401020301ffffaba000", // Second precision, offset -21600
        "0000000156",
        "01",                         // Existing declared end
        "0207ea0a05040501000050dc00", // Minute precision, offset +20700
        "0000000353c3b1"
    );
    assert_vector(
        value,
        62,
        expected,
        "316b15426d4abaab05d28733c3f671067958de140bb047b0e22d729d429c4f6c",
    );
}

#[test]
fn explicit_unknown_end_matches_independent_mcval1_bytes_and_sha256() {
    let value = MeasureCorrectionValues::new(
        note("C"),
        MeasureValidity::new(unknown("U"), note("V"), Some(unknown("E"))).unwrap(),
        note("S"),
    );
    let expected = concat!(
        "4d4356414c31",
        "0000000143",
        "00000019",
        "4d56414c31",
        "00010000000155", // Unknown start and reason U
        "0000000156",
        "01",             // Explicit unknown end is present
        "00010000000145", // Unknown end and reason E
        "0000000153"
    );
    assert_vector(
        value,
        45,
        expected,
        "aa1ae078eabd5db3d2fca10632936b85db498506351ba94c54968e22a1b30ac8",
    );
}

#[test]
fn canonical_bytes_bind_every_permitted_text_and_time_declaration() {
    let start = known(Declared::second(day(), 1, 2, 3, None).unwrap());
    let end = unknown("End omitted in source");
    let baseline = MeasureValidity::new(start.clone(), note("V"), Some(end.clone())).unwrap();
    let expected =
        MeasureCorrectionValues::new(note("C"), baseline.clone(), note("S")).canonical_bytes();
    for changed in [
        MeasureCorrectionValues::new(note("Other C"), baseline.clone(), note("S")),
        MeasureCorrectionValues::new(note("C"), baseline.clone(), note("Other S")),
    ] {
        assert_ne!(changed.canonical_bytes(), expected);
    }
    let start_variants = [
        unknown("Start unknown"),
        known(Declared::date(day(), None).unwrap()),
        known(Declared::minute(day(), 1, 2, None).unwrap()),
        known(Declared::second("2025-10-04".parse().unwrap(), 1, 2, 3, None).unwrap()),
        known(Declared::second("2026-09-04".parse().unwrap(), 1, 2, 3, None).unwrap()),
        known(Declared::second("2026-10-05".parse().unwrap(), 1, 2, 3, None).unwrap()),
        known(Declared::second(day(), 2, 2, 3, None).unwrap()),
        known(Declared::second(day(), 1, 3, 3, None).unwrap()),
        known(Declared::second(day(), 1, 2, 4, None).unwrap()),
        known(Declared::second(day(), 1, 2, 3, Some(UtcOffset::UTC)).unwrap()),
        known(
            Declared::second(day(), 1, 2, 3, Some(UtcOffset::from_hms(-6, 0, 0).unwrap())).unwrap(),
        ),
    ];
    let mut validities: Vec<_> = start_variants
        .into_iter()
        .map(|start| MeasureValidity::new(start, note("V"), Some(end.clone())).unwrap())
        .collect();
    validities.extend([
        MeasureValidity::new(start.clone(), note("Other V"), Some(end)).unwrap(),
        MeasureValidity::new(start.clone(), note("V"), Some(unknown("Other end reason"))).unwrap(),
        MeasureValidity::new(start.clone(), note("V"), None).unwrap(),
        MeasureValidity::new(
            start,
            note("V"),
            Some(known(Declared::date(day(), None).unwrap())),
        )
        .unwrap(),
    ]);
    for validity in validities {
        let changed = MeasureCorrectionValues::new(note("C"), validity, note("S"));
        assert_ne!(changed.canonical_bytes(), expected);
    }
}
