use application::deadline_reevaluation::*;
use domain::{cases::CaseId, crypto::Sha256Digest};
use uuid::Uuid;

fn entry(role: ObservationRole, family: DependencyFamily, id: u128) -> ObservationEntry {
    ObservationEntry {
        role,
        family,
        id: Uuid::from_u128(id),
        revision: 3,
        case_id: None,
        hearing_id: None,
        parent_resolution: None,
        submission_digest: Sha256Digest::from_array([0x11; 32]),
        evidence_digest: Sha256Digest::from_array([0x22; 32]),
    }
}
fn profile_only() -> Observations {
    Observations {
        case_id: CaseId::from_uuid(Uuid::from_u128(1)),
        entries: vec![entry(
            ObservationRole::Profile,
            DependencyFamily::Profile,
            2,
        )],
    }
}
fn notification() -> Observations {
    let mut value = profile_only();
    value.entries[0].case_id = Some(value.case_id);
    let mut source = entry(ObservationRole::Source, DependencyFamily::Notification, 3);
    source.case_id = Some(value.case_id);
    source.parent_resolution = Some(ResolutionReference {
        id: Uuid::from_u128(4),
        revision: 1,
    });
    value.entries.push(source);
    value.entries.push(entry(
        ObservationRole::Calendar,
        DependencyFamily::Calendar,
        5,
    ));
    let mut parent = entry(
        ObservationRole::NotificationParent,
        DependencyFamily::Resolution,
        4,
    );
    parent.case_id = Some(value.case_id);
    parent.revision = 9;
    value.entries.push(parent);
    value
}
fn roundtrip(value: &Observations) -> Vec<u8> {
    let bytes = encode_observations(value).unwrap();
    assert_eq!(decode_observations(&bytes).unwrap(), *value);
    bytes
}

#[test]
fn independent_profile_vector_has_exact_framing_and_digest_offsets() {
    let mut expected = b"DLOB1".to_vec();
    expected.extend(1_u128.to_be_bytes());
    expected.push(1);
    expected.extend([0, 4]);
    expected.extend(2_u128.to_be_bytes());
    expected.extend(3_u32.to_be_bytes());
    expected.extend([0, 0, 0]);
    expected.extend([0x11; 32]);
    expected.extend([0x22; 32]);
    assert_eq!(expected.len(), 111);
    assert_eq!(roundtrip(&profile_only()), expected);
}

#[test]
fn related_resolution_head_does_not_rechain_notification_parent_and_attains_bound() {
    let value = notification();
    let bytes = roundtrip(&value);
    assert_eq!(bytes.len(), 446);
    assert_eq!(MAX_OBSERVATIONS_BYTES, bytes.len());
    assert_eq!(&bytes[5..21], &1_u128.to_be_bytes());
    assert_eq!(bytes[21], 4);
    assert_eq!(&bytes[127..129], &[1, 1]);
    assert_eq!(&bytes[184..188], &1_u32.to_be_bytes());
    assert_eq!(&bytes[359..363], &9_u32.to_be_bytes());
    assert_ne!(
        value.entries[1]
            .parent_resolution
            .as_ref()
            .unwrap()
            .revision,
        value.entries[3].revision
    );
}

#[test]
fn nil_is_a_valid_present_identity_in_every_reference() {
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
    roundtrip(&value);
    value.entries.truncate(1);
    let mut result = entry(ObservationRole::Source, DependencyFamily::HearingResult, 0);
    result.case_id = Some(value.case_id);
    result.hearing_id = Some(Uuid::nil());
    value.entries.push(result);
    roundtrip(&value);
}

#[test]
fn all_fact_families_and_global_or_case_profile_scopes_roundtrip() {
    for family in [
        DependencyFamily::Resolution,
        DependencyFamily::Notification,
        DependencyFamily::HearingResult,
    ] {
        let mut value = profile_only();
        let mut source = entry(ObservationRole::Source, family, 3);
        source.case_id = Some(value.case_id);
        match family {
            DependencyFamily::Notification => {
                source.parent_resolution = Some(ResolutionReference {
                    id: Uuid::from_u128(4),
                    revision: 1,
                })
            }
            DependencyFamily::HearingResult => source.hearing_id = Some(Uuid::from_u128(5)),
            _ => {}
        }
        value.entries.push(source);
        roundtrip(&value);
        value.entries[0].case_id = Some(value.case_id);
        roundtrip(&value);
    }
}

#[test]
fn legacy_notification_can_omit_parent_head_but_not_its_exact_parent_reference() {
    let mut value = notification();
    value.entries.pop();
    roundtrip(&value);
    value.entries[1].parent_resolution = None;
    assert!(encode_observations(&value).is_err());
}

#[test]
fn empty_missing_duplicate_unordered_or_excess_roles_are_invalid() {
    let mut invalid = Vec::new();
    let mut value = profile_only();
    value.entries.clear();
    invalid.push(value);
    let mut value = notification();
    value.entries.remove(0);
    invalid.push(value);
    let mut value = notification();
    value.entries.swap(1, 2);
    invalid.push(value);
    let mut value = profile_only();
    value.entries.push(value.entries[0].clone());
    invalid.push(value);
    let mut value = notification();
    value.entries.push(value.entries[3].clone());
    invalid.push(value);
    for value in invalid {
        assert!(encode_observations(&value).is_err());
    }
}

#[test]
fn family_scope_and_optional_field_shape_are_checked_before_encoding() {
    for mutate in [
        |value: &mut Observations| value.entries[0].family = DependencyFamily::Calendar,
        |value: &mut Observations| value.entries[0].case_id = Some(CaseId::from_uuid(Uuid::nil())),
        |value: &mut Observations| value.entries[0].hearing_id = Some(Uuid::nil()),
        |value: &mut Observations| {
            value.entries[0].parent_resolution = value.entries[1].parent_resolution
        },
        |value: &mut Observations| value.entries[1].family = DependencyFamily::Profile,
        |value: &mut Observations| value.entries[1].case_id = None,
        |value: &mut Observations| value.entries[1].case_id = Some(CaseId::from_uuid(Uuid::nil())),
        |value: &mut Observations| value.entries[1].hearing_id = Some(Uuid::nil()),
        |value: &mut Observations| value.entries[1].revision = 0,
        |value: &mut Observations| value.entries[2].family = DependencyFamily::Resolution,
        |value: &mut Observations| value.entries[2].case_id = Some(value.case_id),
        |value: &mut Observations| value.entries[2].hearing_id = Some(Uuid::nil()),
        |value: &mut Observations| value.entries[3].family = DependencyFamily::Notification,
        |value: &mut Observations| value.entries[3].case_id = None,
    ] {
        let mut value = notification();
        mutate(&mut value);
        assert!(encode_observations(&value).is_err(), "{value:?}");
    }
}

#[test]
fn related_parent_requires_notification_and_same_root_with_valid_revisions() {
    for mutate in [
        |value: &mut Observations| {
            value.entries.remove(1);
        },
        |value: &mut Observations| {
            value.entries[1].family = DependencyFamily::Resolution;
            value.entries[1].parent_resolution = None;
        },
        |value: &mut Observations| value.entries[3].id = Uuid::nil(),
        |value: &mut Observations| value.entries[3].revision = 0,
        |value: &mut Observations| {
            value.entries[1]
                .parent_resolution
                .as_mut()
                .unwrap()
                .revision = 0
        },
        |value: &mut Observations| {
            value.entries[1]
                .parent_resolution
                .as_mut()
                .unwrap()
                .revision = 10
        },
    ] {
        let mut value = notification();
        mutate(&mut value);
        assert!(encode_observations(&value).is_err());
    }
}

#[test]
fn all_truncations_versions_trailing_bytes_and_oversize_inputs_fail_closed() {
    for value in [profile_only(), notification()] {
        let bytes = roundtrip(&value);
        for length in 0..bytes.len() {
            assert!(decode_observations(&bytes[..length]).is_err(), "{length}");
        }
        let mut invalid = bytes.clone();
        invalid.push(0);
        assert!(decode_observations(&invalid).is_err());
        for version in [0, b'0', b'2'] {
            let mut invalid = bytes.clone();
            invalid[4] = version;
            assert!(decode_observations(&invalid).is_err());
        }
    }
    assert!(decode_observations(&vec![0; 513]).is_err());
}

#[test]
fn unknown_discriminants_presence_tags_and_false_counts_are_rejected() {
    let bytes = roundtrip(&profile_only());
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
        let mut invalid = bytes.clone();
        invalid[offset] = byte;
        assert!(decode_observations(&invalid).is_err(), "{offset}={byte}");
    }
    let mut invalid = bytes;
    invalid[40..44].copy_from_slice(&0_u32.to_be_bytes());
    assert!(decode_observations(&invalid).is_err());
}

#[test]
fn decoder_enforces_scope_order_and_related_parent_without_silent_normalization() {
    let bytes = roundtrip(&notification());
    for (offset, byte) in [(23, 3), (46, 2), (127, 0), (128, 4), (168, 1), (342, 1)] {
        let mut invalid = bytes.clone();
        invalid[offset] = byte;
        if invalid == bytes {
            continue;
        }
        assert!(decode_observations(&invalid).is_err(), "{offset}={byte}");
    }
    let mut invalid = bytes.clone();
    invalid[359..363].copy_from_slice(&0_u32.to_be_bytes());
    assert!(decode_observations(&invalid).is_err());
    let mut invalid = bytes;
    invalid[184..188].copy_from_slice(&10_u32.to_be_bytes());
    assert!(decode_observations(&invalid).is_err());
}
