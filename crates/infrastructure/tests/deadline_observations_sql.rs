mod case_administration_support;
mod deadline_observations_sql_support;

use application::deadline_reevaluation::*;
use deadline_observations_sql_support::*;
use domain::cases::CaseId;
use serde_json::json;
use uuid::Uuid;

#[test]
fn independent_global_profile_vector_preserves_exact_json_shape_and_lowercase_uuid() {
    let Some(mut db) = fixture() else { return };
    let mut bytes = b"DLOB1".to_vec();
    bytes.extend(1_u128.to_be_bytes());
    bytes.extend([1, 0, 4]);
    bytes.extend(0xabcd_u128.to_be_bytes());
    bytes.extend(3_u32.to_be_bytes());
    bytes.extend([0, 0, 0]);
    bytes.extend([0x11; 32]);
    bytes.extend([0x22; 32]);
    assert_eq!(bytes.len(), 111);
    assert_eq!(parity(&mut db, &profile()), bytes);
    assert_eq!(
        parse(&mut db, &bytes),
        json!({
            "case_id": "00000000-0000-0000-0000-000000000001",
            "entries": [{"role":"profile", "family":"profile",
                "id":"00000000-0000-0000-0000-00000000abcd", "revision":3,
                "case_id":null, "hearing_id":null, "parent_resolution":null,
                "submission_digest":"11".repeat(32), "evidence_digest":"22".repeat(32)}]
        })
    );
}

#[test]
fn legacy_observations_preserve_optional_parent_absence_and_every_source_family() {
    let Some(mut db) = fixture() else { return };
    for private in [false, true] {
        for family in [
            DependencyFamily::Resolution,
            DependencyFamily::Notification,
            DependencyFamily::HearingResult,
        ] {
            let mut value = profile();
            if private {
                value.entries[0].case_id = Some(value.case_id);
            }
            let mut source = entry(ObservationRole::Source, family, 3);
            source.case_id = Some(value.case_id);
            if family == DependencyFamily::Notification {
                source.parent_resolution = Some(ResolutionReference {
                    id: Uuid::from_u128(4),
                    revision: 1,
                });
            }
            if family == DependencyFamily::HearingResult {
                source.hearing_id = Some(Uuid::from_u128(7));
            }
            value.entries.push(source);
            parity(&mut db, &value);
        }
    }
    let mut legacy = notification();
    legacy.entries.pop();
    let bytes = parity(&mut db, &legacy);
    let view = parse(&mut db, &bytes);
    assert_eq!(view["entries"].as_array().unwrap().len(), 3);
    assert_eq!(view["entries"][1]["parent_resolution"]["revision"], 1);
}

#[test]
fn maximal_notification_frame_preserves_exact_parent_separate_from_observed_parent_head() {
    let Some(mut db) = fixture() else { return };
    let value = notification();
    let bytes = parity(&mut db, &value);
    assert_eq!(bytes.len(), 446);
    assert_eq!(bytes.len(), MAX_OBSERVATIONS_BYTES);
    let view = parse(&mut db, &bytes);
    assert_eq!(view["entries"][1]["parent_resolution"]["revision"], 1);
    assert_eq!(view["entries"][3]["revision"], 9);
    assert_eq!(view["entries"][3]["family"], "resolution");
}

#[test]
fn present_nil_identifiers_and_unsigned_revision_maxima_are_not_null_or_signed() {
    let Some(mut db) = fixture() else { return };
    let mut value = notification();
    value.case_id = CaseId::from_uuid(Uuid::nil());
    for entry in &mut value.entries {
        entry.id = Uuid::nil();
        entry.revision = u32::MAX;
        if entry.case_id.is_some() {
            entry.case_id = Some(value.case_id);
        }
        if let Some(parent) = &mut entry.parent_resolution {
            parent.id = Uuid::nil();
            parent.revision = u32::MAX;
        }
    }
    parity(&mut db, &value);
    value.entries.truncate(1);
    value.entries[0].case_id = None;
    let mut hearing = entry(ObservationRole::Source, DependencyFamily::HearingResult, 0);
    hearing.case_id = Some(value.case_id);
    hearing.hearing_id = Some(Uuid::nil());
    hearing.revision = u32::MAX;
    value.entries.push(hearing);
    let bytes = parity(&mut db, &value);
    let view = parse(&mut db, &bytes);
    assert_eq!(view["entries"][1]["hearing_id"], Uuid::nil().to_string());
    assert_eq!(view["entries"][1]["revision"], json!(4_294_967_295_u64));
    assert!(view["entries"][0]["case_id"].is_null());
}

#[test]
fn role_order_presence_duplicates_and_collection_bounds_are_rejected_without_normalizing() {
    let Some(mut db) = fixture() else { return };
    for change in 0..6 {
        let mut value = notification();
        match change {
            0 => value.entries.clear(),
            1 => {
                value.entries.remove(0);
            }
            2 => value.entries.swap(1, 2),
            3 => {
                value.entries.truncate(1);
                value.entries.push(value.entries[0].clone());
            }
            4 => value.entries.push(value.entries[3].clone()),
            _ => {
                value.entries[1].role = ObservationRole::Profile;
            }
        }
        rejected(&mut db, &wire(&value));
    }
}

#[test]
fn family_scope_and_optional_fields_match_the_rust_contract() {
    let Some(mut db) = fixture() else { return };
    for change in 0..17 {
        let mut value = notification();
        match change {
            0 => value.entries[0].family = DependencyFamily::Calendar,
            1 => value.entries[0].case_id = Some(CaseId::from_uuid(Uuid::nil())),
            2 => value.entries[0].hearing_id = Some(Uuid::nil()),
            3 => value.entries[0].parent_resolution = value.entries[1].parent_resolution,
            4 => value.entries[1].family = DependencyFamily::Profile,
            5 => value.entries[1].case_id = None,
            6 => value.entries[1].case_id = Some(CaseId::from_uuid(Uuid::nil())),
            7 => value.entries[1].hearing_id = Some(Uuid::nil()),
            8 => value.entries[1].revision = 0,
            9 => value.entries[2].family = DependencyFamily::Resolution,
            10 => value.entries[2].case_id = Some(value.case_id),
            11 => value.entries[2].hearing_id = Some(Uuid::nil()),
            12 => value.entries[3].family = DependencyFamily::Notification,
            13 => value.entries[3].case_id = None,
            14 => {
                value.entries.truncate(2);
                value.entries[1].family = DependencyFamily::HearingResult;
                value.entries[1].parent_resolution = None;
            }
            15 => value.entries[1].parent_resolution = None,
            _ => value.entries[2].revision = 0,
        }
        rejected(&mut db, &wire(&value));
    }
}

#[test]
fn parent_head_requires_a_notification_same_root_and_nonzero_covering_revision() {
    let Some(mut db) = fixture() else { return };
    for change in 0..6 {
        let mut value = notification();
        match change {
            0 => {
                value.entries.remove(1);
            }
            1 => {
                value.entries[1].family = DependencyFamily::Resolution;
                value.entries[1].parent_resolution = None;
            }
            2 => value.entries[3].id = Uuid::nil(),
            3 => value.entries[3].revision = 0,
            4 => {
                value.entries[1]
                    .parent_resolution
                    .as_mut()
                    .unwrap()
                    .revision = 0
            }
            _ => {
                value.entries[1]
                    .parent_resolution
                    .as_mut()
                    .unwrap()
                    .revision = 10
            }
        }
        rejected(&mut db, &wire(&value));
    }
}

#[test]
fn every_truncation_unknown_version_trailing_byte_and_oversize_frame_is_rejected() {
    let Some(mut db) = fixture() else { return };
    for value in [profile(), notification()] {
        let bytes = parity(&mut db, &value);
        for length in 0..bytes.len() {
            rejected(&mut db, &bytes[..length]);
        }
        for version in [0, b'0', b'2'] {
            let mut bad = bytes.clone();
            bad[4] = version;
            rejected(&mut db, &bad);
        }
        let mut bad = bytes;
        bad.push(0);
        rejected(&mut db, &bad);
    }
    rejected(&mut db, &vec![0; 447]);
}

#[test]
fn invalid_tags_false_counts_presence_flags_and_zero_revision_are_rejected() {
    let Some(mut db) = fixture() else { return };
    let bytes = parity(&mut db, &profile());
    for (offset, byte) in [
        (21, 0),
        (21, 2),
        (21, 5),
        (21, 255),
        (22, 4),
        (23, 5),
        (44, 2),
        (45, 2),
        (46, 2),
    ] {
        let mut bad = bytes.clone();
        bad[offset] = byte;
        rejected(&mut db, &bad);
    }
    let mut bad = bytes;
    bad[40..44].copy_from_slice(&0_u32.to_be_bytes());
    rejected(&mut db, &bad);
    let bytes = parity(&mut db, &notification());
    for offset in [44, 61, 62, 149, 166, 167, 274, 275, 276, 363, 380, 381] {
        let mut bad = bytes.clone();
        bad[offset] = 2;
        rejected(&mut db, &bad);
    }
}
