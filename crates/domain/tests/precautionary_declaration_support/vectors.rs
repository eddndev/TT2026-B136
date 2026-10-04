use domain::precautionary_measures::{MeasureDecisionValues, MeasureSupervision, MeasureValues};

use crate::precautionary_declaration_support::*;

#[test]
fn known_supervision_matches_independent_meas1_golden_vector() {
    let expected = concat!(
        "4d45415331",                       // MEAS1
        "00000000000000000000000000000001", // Subject UUID
        "00000002",                         // Exact subject revision
        "1111111111111111111111111111111111111111111111111111111111111111",
        "0d",                               // Pretrial detention
        "0000000343c3a9",                   // Three UTF-8 bytes in conditions
        "00000012",                         // 18-byte unchanged MVAL1
        "4d56414c31",                       // MVAL1
        "0107ea0a040000",                   // Date 2026-10-04, no offset or reason
        "0000000156",                       // Validity statement V
        "00",                               // No declared end
        "00",                               // Known supervision
        "00000000000000000000000000000003", // Exact participant UUID
        "00000004",                         // Exact participant revision
        "0000000153"                        // Statement S
    );
    let values = MeasureValues::new(measure_input());
    assert_eq!(values.canonical_bytes().len(), 113);
    assert_eq!(hex(&values.canonical_bytes()), expected);
}

#[test]
fn unknown_supervision_matches_independent_meas1_golden_vector() {
    let expected = concat!(
        "4d45415331",
        "00000000000000000000000000000001",
        "00000002",
        "1111111111111111111111111111111111111111111111111111111111111111",
        "0d",
        "0000000343c3a9",
        "00000012",
        "4d56414c31",
        "0107ea0a040000",
        "0000000156",
        "00",
        "01",         // Unknown supervision
        "0000000155"  // Required reason U; no fabricated participant
    );
    let mut input = measure_input();
    input.supervision = MeasureSupervision::Unknown { reason: note("U") };
    let values = MeasureValues::new(input);
    assert_eq!(values.canonical_bytes().len(), 93);
    assert_eq!(hex(&values.canonical_bytes()), expected);
}

#[test]
fn exact_decision_time_matches_independent_mdval1_golden_vector() {
    let expected = concat!(
        "4d4456414c31",                     // MDVAL1
        "000000034ac3a9",                   // Three UTF-8 bytes in authority
        "0307ea0a04010203",                 // Second precision, original components
        "01ffffaba0",                       // Explicit offset -21600 seconds
        "00",                               // No unknown-time reason
        "000000014a",                       // Justification J
        "00000000000000000000000000000005", // Exact support document UUID
        "00000006",                         // Exact content version
        "2222222222222222222222222222222222222222222222222222222222222222",
        "000000014c" // Source locator L
    );
    let values = MeasureDecisionValues::new(decision_input());
    assert_eq!(values.canonical_bytes().len(), 89);
    assert_eq!(hex(&values.canonical_bytes()), expected);
}

#[test]
fn unknown_decision_time_matches_independent_mdval1_golden_vector() {
    let expected = concat!(
        "4d4456414c31",
        "000000034ac3a9",
        "00",           // Unknown precision
        "010000000155", // Required reason U
        "000000014a",
        "00000000000000000000000000000005",
        "00000006",
        "2222222222222222222222222222222222222222222222222222222222222222",
        "000000014c"
    );
    let mut input = decision_input();
    input.declared_at = unknown_time("U");
    let values = MeasureDecisionValues::new(input);
    assert_eq!(values.canonical_bytes().len(), 82);
    assert_eq!(hex(&values.canonical_bytes()), expected);
}
