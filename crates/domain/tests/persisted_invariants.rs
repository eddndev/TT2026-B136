use domain::crypto::{DocumentVersion, RecoveryCodeSet, RECOVERY_CODE_COUNT};
use domain::DomainError;
use serde_json::json;

#[test]
fn document_version_deserialization_rejects_zero() {
    assert!(serde_json::from_str::<DocumentVersion>("0").is_err());
}

#[test]
fn valid_document_versions_keep_their_numeric_wire_format() {
    for value in [1, 42, u32::MAX] {
        let version = DocumentVersion::new(value).unwrap();
        let encoded = serde_json::to_string(&version).unwrap();
        assert_eq!(encoded, value.to_string());
        assert_eq!(
            serde_json::from_str::<DocumentVersion>(&encoded).unwrap(),
            version
        );
    }
}

#[test]
fn maximum_document_version_returns_an_error_without_wrapping() {
    let version = DocumentVersion::new(u32::MAX).unwrap();
    assert_eq!(version.next(), Err(DomainError::DocumentVersionExhausted));
    assert_eq!(version.get(), u32::MAX);
    assert_eq!(
        DocumentVersion::new(u32::MAX - 1).unwrap().next().unwrap(),
        version
    );
}

#[test]
fn persisted_recovery_sets_require_exactly_the_issued_slot_count() {
    for count in [0, 1, RECOVERY_CODE_COUNT - 1, RECOVERY_CODE_COUNT + 1] {
        let encoded = json!({"slots": vec![Some("stored-phc-hash"); count]});
        assert!(serde_json::from_value::<RecoveryCodeSet>(encoded).is_err());
    }
}

#[test]
fn consumed_recovery_slots_survive_round_trip_without_becoming_available() {
    for consumed in 0..=RECOVERY_CODE_COUNT {
        let slots: Vec<_> = (0..RECOVERY_CODE_COUNT)
            .map(|index| (index >= consumed).then(|| format!("stored-phc-{index}")))
            .collect();
        let encoded = json!({"slots": slots});
        let restored = serde_json::from_value::<RecoveryCodeSet>(encoded.clone()).unwrap();
        assert_eq!(restored.remaining(), RECOVERY_CODE_COUNT - consumed);
        assert_eq!(serde_json::to_value(&restored).unwrap(), encoded);
    }
}

#[test]
fn initial_recovery_sets_keep_the_existing_object_wire_format() {
    let hashes: Vec<_> = (0..RECOVERY_CODE_COUNT)
        .map(|index| format!("hash-{index}"))
        .collect();
    let recovery = RecoveryCodeSet::from_hashes(hashes.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(&recovery).unwrap(),
        json!({"slots": hashes})
    );
}
