mod case_administration_support;
mod deadline_observations_sql_support;
mod deadline_tracking_sql_support;

use deadline_tracking_sql_support::*;
use serde_json::{json, Value};

#[test]
fn independent_legacy_r0_suffix_preserves_all_policies_and_digest_fields() {
    let Some(mut db) = fixture() else { return };
    let mut bytes = vec![0, 0, 0, 0, 0];
    bytes.extend([0x11; 32]);
    bytes.push(0);
    bytes.extend([0x22; 32]);
    bytes.extend([0x33; 32]);
    assert_eq!(bytes.len(), 102);
    assert_eq!(bytes, frame([0; 3], 0, &[], [0x11; 32], None));
    assert_eq!(
        parse(&mut db, &bytes),
        json!({
            "policies":{"profile":"undetermined","source":"undetermined","calendar":"undetermined"},
            "review":{"state":"legacy_undeclared","reasons":[]},
            "observations_digest":"11".repeat(32),"administration_revision":null,
            "administration_values_digest":"22".repeat(32),"administration_evidence_digest":"33".repeat(32)
        })
    );
}

#[test]
fn accepted_suffix_preserves_r1_and_unsigned_administration_maximum() {
    let Some(mut db) = fixture() else { return };
    for revision in [1, 0x0102_0304, u32::MAX] {
        let bytes = frame([2, 1, 2], 1, &[], [0xab; 32], Some(revision));
        assert_eq!(bytes.len(), 106);
        assert_eq!(&bytes[38..42], &revision.to_be_bytes());
        assert_eq!(
            parse(&mut db, &bytes),
            expected([2, 1, 2], 1, &[], [0xab; 32], Some(revision))
        );
    }
}

#[test]
fn pending_suffix_preserves_ordered_reasons_and_fixed_retirement() {
    let Some(mut db) = fixture() else { return };
    for (policies, reasons) in [
        ([2, 1, 0], vec![[0, 1], [1, 2], [2, 3]]),
        ([1, 1, 1], vec![[0, 2], [1, 2], [2, 2]]),
        (
            [0, 0, 0],
            vec![[0, 2], [0, 3], [1, 2], [1, 3], [2, 2], [2, 3]],
        ),
    ] {
        let bytes = frame(policies, 2, &reasons, [0x11; 32], Some(1));
        assert_eq!(bytes.len(), 106 + 2 * reasons.len());
        assert_eq!(
            parse(&mut db, &bytes),
            expected(policies, 2, &reasons, [0x11; 32], Some(1))
        );
    }
}

#[test]
fn null_suffix_is_null_and_optional_dependency_presence_is_not_invented() {
    let Some(mut db) = fixture() else { return };
    let parsed: Option<Value> = db
        .admin
        .query_one("SELECT deadline_tracking(NULL::bytea)", &[])
        .unwrap()
        .get(0);
    assert!(parsed.is_none());
    for (policies, state, reasons) in [
        ([1, 0, 0], 1, vec![]),
        ([2, 0, 0], 2, vec![[0, 1]]),
        ([0, 0, 0], 2, vec![[0, 3]]),
    ] {
        let bytes = frame(policies, state, &reasons, [0x11; 32], None);
        assert_eq!(
            parse(&mut db, &bytes),
            expected(policies, state, &reasons, [0x11; 32], None)
        );
    }
}

#[test]
fn every_truncation_trailing_byte_and_prefixed_or_oversize_suffix_is_rejected() {
    let Some(mut db) = fixture() else { return };
    for bytes in [
        frame([0; 3], 0, &[], [0x11; 32], None),
        frame([2, 1, 2], 1, &[], [0x11; 32], Some(1)),
        frame(
            [0; 3],
            2,
            &[[0, 2], [0, 3], [1, 2], [1, 3], [2, 2], [2, 3]],
            [0x11; 32],
            Some(1),
        ),
    ] {
        parse(&mut db, &bytes);
        for end in 0..bytes.len() {
            rejected(&mut db, &bytes[..end]);
        }
        let mut extra = bytes.clone();
        extra.push(0);
        rejected(&mut db, &extra);
        let mut prefixed = b"DLST2".to_vec();
        prefixed.extend(bytes);
        rejected(&mut db, &prefixed);
    }
    rejected(&mut db, &[0; 123]);
    let eight = [
        [0, 1],
        [0, 2],
        [0, 3],
        [1, 0],
        [1, 2],
        [1, 3],
        [2, 2],
        [2, 3],
    ];
    let bytes = frame([0; 3], 2, &eight, [0x11; 32], Some(1));
    assert_eq!(bytes.len(), 122);
    rejected(&mut db, &bytes);
}

#[test]
fn unknown_tags_reason_counts_presence_and_zero_administration_are_rejected() {
    let Some(mut db) = fixture() else { return };
    let bytes = frame([1, 0, 0], 1, &[], [0x11; 32], Some(1));
    for (offset, value) in [(0, 3), (1, 3), (2, 3), (3, 3), (4, 9), (37, 2)] {
        let mut changed = bytes.clone();
        changed[offset] = value;
        rejected(&mut db, &changed);
    }
    rejected(&mut db, &frame([1, 0, 0], 1, &[], [0x11; 32], Some(0)));
    let mut missing = bytes.clone();
    missing[37] = 0;
    rejected(&mut db, &missing);
    let mut invented = frame([1, 0, 0], 1, &[], [0x11; 32], None);
    invented[37] = 1;
    rejected(&mut db, &invented);
    for reasons in [&[[3, 2]][..], &[[0, 4]][..]] {
        rejected(&mut db, &frame([1; 3], 2, reasons, [0x11; 32], None));
    }
}

#[test]
fn review_state_reason_order_and_dependency_are_validated_without_normalizing() {
    let Some(mut db) = fixture() else { return };
    for (policies, state, reasons) in [
        ([0; 3], 0, vec![[0, 3]]),
        ([1, 0, 0], 0, vec![]),
        ([1; 3], 1, vec![[0, 2]]),
        ([1; 3], 2, vec![]),
        ([2; 3], 2, vec![[0, 0]]),
        ([2; 3], 2, vec![[1, 1]]),
        ([2; 3], 2, vec![[2, 0]]),
        ([2; 3], 2, vec![[2, 1]]),
        ([2; 3], 2, vec![[1, 0], [0, 1]]),
        ([2; 3], 2, vec![[0, 2], [0, 1]]),
        ([2; 3], 2, vec![[0, 1], [0, 1]]),
    ] {
        rejected(&mut db, &frame(policies, state, &reasons, [0x11; 32], None));
    }
}

#[test]
fn policy_incompatible_reasons_and_undeclared_profile_acceptance_are_rejected() {
    let Some(mut db) = fixture() else { return };
    for (policies, state, reasons) in [
        ([0; 3], 1, vec![]),
        ([0, 1, 1], 2, vec![[1, 2]]),
        ([1; 3], 2, vec![[0, 1]]),
        ([1; 3], 2, vec![[1, 0]]),
        ([0; 3], 2, vec![[0, 1], [0, 3]]),
        ([1; 3], 2, vec![[0, 3]]),
        ([2; 3], 2, vec![[1, 3]]),
        ([1; 3], 2, vec![[2, 3]]),
    ] {
        rejected(&mut db, &frame(policies, state, &reasons, [0x11; 32], None));
    }
}
