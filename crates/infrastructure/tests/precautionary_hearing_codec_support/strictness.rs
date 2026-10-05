use super::*;

fn rejected(mutate: impl FnOnce(&mut Value)) {
    let original = fixture(true);
    let mut projection = view(&original);
    mutate(&mut projection);
    assert!(values(&original.canonical_bytes(), &projection).is_err());
}

#[test]
fn missing_or_unknown_fields_are_rejected_at_every_object_level() {
    for path in [
        "",
        "/time",
        "/scheduling_basis",
        "/participants/0",
        "/review_targets/0",
    ] {
        let original = fixture(true);
        let projection = view(&original);
        let fields: Vec<_> = projection
            .pointer(path)
            .unwrap()
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        for field in fields {
            let mut changed = projection.clone();
            changed
                .pointer_mut(path)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(&field);
            assert!(
                values(&original.canonical_bytes(), &changed).is_err(),
                "{path}/{field}"
            );
        }
        rejected(|p| {
            p.pointer_mut(path)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert("unexpected".into(), json!(0));
        });
    }
}

#[test]
fn incorrect_projection_shapes_and_scalar_types_are_rejected() {
    for path in [
        "",
        "/time",
        "/scheduling_basis",
        "/participants",
        "/review_targets",
        "/participants/0",
        "/review_targets/0",
        "/purpose",
        "/modality",
        "/venue",
        "/time/seconds",
        "/time/offset_seconds",
        "/participants/0/revision",
        "/scheduling_basis/version",
        "/review_targets/0/revision",
    ] {
        rejected(|p| *p.pointer_mut(path).unwrap() = Value::Null);
    }
    rejected(|p| p["note"] = json!(42));
    rejected(|p| p["purpose"] = json!("Review"));
    rejected(|p| p["modality"] = json!("remote"));
    rejected(|p| p["time"]["seconds"] = json!("0"));
    rejected(|p| p["time"]["seconds"] = json!(0.0));
}

#[test]
fn persisted_text_cannot_be_repaired_by_trimming_or_newline_normalization() {
    for path in [
        "/venue",
        "/note",
        "/scheduling_basis/statement",
        "/scheduling_basis/locator",
    ] {
        rejected(|p| {
            let old = p.pointer(path).unwrap().as_str().unwrap();
            let changed = format!(" {old} ");
            *p.pointer_mut(path).unwrap() = json!(changed);
        });
    }
    let mut source = input(false);
    source.note = Some(HearingNote::new("First\nSecond").unwrap());
    let original = PrecautionaryHearingValues::new(source).unwrap();
    let mut projection = view(&original);
    projection["note"] = json!("First\r\nSecond");
    assert!(values(&original.canonical_bytes(), &projection).is_err());
    rejected(|p| p["note"] = json!(""));
    rejected(|p| p["venue"] = json!("Court\tA"));
}

#[test]
fn uuid_and_digest_text_must_use_exact_canonical_representations() {
    rejected(|p| {
        p["scheduling_basis"]["document_id"] = json!("00000000-0000-0000-0000-00000000004D")
    });
    rejected(|p| p["review_targets"][0]["id"] = json!("00000000-0000-0000-0000-00000000000B"));
    rejected(|p| p["participants"][0]["id"] = json!("00000000000000000000000000000009"));
    rejected(|p| p["review_targets"][0]["digest"] = json!("55".repeat(31)));
    let mut source = input(true);
    source.review_targets[0] = PrecautionaryMeasureRef::new(
        MeasureId::from_uuid(Uuid::from_u128(11)),
        MeasureRevision::new(3).unwrap(),
        Sha256Digest::from_array([0xab; 32]),
    );
    let original = PrecautionaryHearingValues::new(source).unwrap();
    let mut projection = view(&original);
    projection["review_targets"][0]["digest"] = json!("AB".repeat(32));
    assert!(values(&original.canonical_bytes(), &projection).is_err());
}

#[test]
fn sorted_exact_reference_arrays_cannot_be_reordered_or_duplicated() {
    let mut source = input(true);
    source.participants.push(participant(10));
    source.review_targets.push(target(12));
    let original = PrecautionaryHearingValues::new(source).unwrap();
    for key in ["participants", "review_targets"] {
        let mut projection = view(&original);
        projection[key].as_array_mut().unwrap().reverse();
        assert!(values(&original.canonical_bytes(), &projection).is_err());
        let mut projection = view(&original);
        projection[key][1] = projection[key][0].clone();
        assert!(values(&original.canonical_bytes(), &projection).is_err());
    }
}

#[test]
fn oversized_canonical_and_projection_components_fail_without_truncation() {
    let original = fixture(true);
    let mut oversized = original.canonical_bytes();
    oversized.resize(16_396, 0);
    assert!(values(&oversized, &Value::Null).is_err());
    for key in ["participants", "review_targets"] {
        rejected(|p| p[key] = json!(vec![p[key][0].clone(); 33]));
    }
    rejected(|p| p["venue"] = json!("x".repeat(501)));
    for path in [
        "/note",
        "/scheduling_basis/statement",
        "/scheduling_basis/locator",
    ] {
        rejected(|p| *p.pointer_mut(path).unwrap() = json!("x".repeat(1001)));
    }
    rejected(|p| p["venue"] = json!("x".repeat(65_537)));
}

#[test]
fn revision_and_document_version_numbers_are_positive_bounded_integers() {
    for path in [
        "/participants/0/revision",
        "/scheduling_basis/version",
        "/review_targets/0/revision",
    ] {
        for bad in [
            json!(0),
            json!(-1),
            json!(4_294_967_296u64),
            json!(2.5),
            json!("2"),
        ] {
            rejected(|p| *p.pointer_mut(path).unwrap() = bad);
        }
    }
}

#[test]
fn temporal_projection_rejects_unsupported_precision_offsets_and_years() {
    for offset in [1, -50_401, 50_401, 2_147_483_648i64] {
        rejected(|p| p["time"]["offset_seconds"] = json!(offset));
    }
    for seconds in [-62_135_596_801i64, 253_402_300_800, i64::MAX] {
        rejected(|p| p["time"]["seconds"] = json!(seconds));
    }
    rejected(|p| p["time"]["nanoseconds"] = json!(1));
}

#[test]
fn purpose_target_constraints_are_checked_by_the_persisted_codec() {
    rejected(|p| p["purpose"] = json!("imposition"));
    rejected(|p| p["review_targets"] = json!([]));
}

#[test]
fn altered_truncated_extended_and_wrong_family_bytes_are_rejected() {
    let original = fixture(true);
    let projection = view(&original);
    let canonical = original.canonical_bytes();
    for length in [0, 5, canonical.len() - 1] {
        assert!(values(&canonical[..length], &projection).is_err());
    }
    let mut appended = canonical.clone();
    appended.push(0);
    assert!(values(&appended, &projection).is_err());
    for index in [0, 6, 14, 18, 19, canonical.len() - 1] {
        let mut changed = canonical.clone();
        changed[index] ^= 1;
        assert!(values(&changed, &projection).is_err());
    }
}

#[test]
fn a_valid_projection_for_different_values_cannot_replace_committed_values() {
    let original = fixture(true);
    let mut source = input(true);
    source.scheduling_basis = PrecautionaryHearingSchedulingBasis::new(
        HearingNote::new("Set by order").unwrap(),
        source.scheduling_basis.support(),
        HearingNote::new("Page 3").unwrap(),
    );
    let changed = PrecautionaryHearingValues::new(source).unwrap();
    assert!(values(&original.canonical_bytes(), &view(&changed)).is_err());
}
