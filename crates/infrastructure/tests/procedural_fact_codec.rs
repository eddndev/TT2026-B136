mod procedural_fact_codec_support;
use application::procedural_facts::{fact_sources_bytes, ProceduralFactValues};
use infrastructure::procedural_fact_codec as codec;
use procedural_fact_codec_support::*;
use serde_json::json;

#[test]
fn rebuilds_all_independent_resolution_and_notification_vectors() {
    let vectors = value_vectors();
    assert_eq!(vectors.len(), 28);
    for vector in vectors {
        let canonical = bytes(&vector);
        let result = codec::values(
            vector["family"].as_str().unwrap(),
            &canonical,
            &vector["normalized"],
        )
        .unwrap_or_else(|e| panic!("{}: {e}", vector["name"]));
        let encoded = match result {
            ProceduralFactValues::Resolution(value) => value.canonical_bytes(),
            ProceduralFactValues::Notification(value) => value.canonical_bytes(),
        };
        assert_eq!(encoded, canonical, "{}", vector["name"]);
    }
}

#[test]
fn rebuilds_all_independent_source_vectors() {
    let vectors = source_vectors();
    assert_eq!(vectors.len(), 25);
    for vector in vectors {
        let canonical = bytes(&vector);
        let result = codec::sources(&canonical, &vector["sources"])
            .unwrap_or_else(|e| panic!("{}: {e}", vector["name"]));
        assert_eq!(
            fact_sources_bytes(&result).unwrap(),
            canonical,
            "{}",
            vector["name"]
        );
    }
}

#[test]
fn every_value_object_rejects_unknown_and_missing_fields_including_null_options() {
    for vector in value_vectors() {
        let original = &vector["normalized"];
        for path in object_paths(original) {
            let mut changed = original.clone();
            changed
                .pointer_mut(&path)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert("unexpected".into(), json!(1));
            reject_value(&vector, &changed);
            for key in original.pointer(&path).unwrap().as_object().unwrap().keys() {
                let mut changed = original.clone();
                changed
                    .pointer_mut(&path)
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .remove(key);
                reject_value(&vector, &changed);
            }
        }
    }
}

#[test]
fn every_source_object_rejects_unknown_and_missing_fields_including_null_options() {
    for vector in source_vectors() {
        let original = &vector["sources"];
        for path in object_paths(original) {
            let mut changed = original.clone();
            changed
                .pointer_mut(&path)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert("unexpected".into(), json!(1));
            reject_source(&vector, &changed);
            for key in original.pointer(&path).unwrap().as_object().unwrap().keys() {
                let mut changed = original.clone();
                changed
                    .pointer_mut(&path)
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .remove(key);
                reject_source(&vector, &changed);
            }
        }
    }
}

#[test]
fn values_reject_truncated_trailing_damaged_or_wrong_family_canonical_bytes() {
    for vector in value_vectors() {
        let canonical = bytes(&vector);
        let family = vector["family"].as_str().unwrap();
        let projection = &vector["normalized"];
        for length in [0, 5, canonical.len() / 2, canonical.len() - 1] {
            inconsistent(codec::values(family, &canonical[..length], projection));
        }
        let mut changed = canonical.clone();
        changed.push(0);
        inconsistent(codec::values(family, &changed, projection));
        let last = changed.len() - 2;
        changed[last] ^= 1;
        changed.pop();
        inconsistent(codec::values(family, &changed, projection));
        for wrong in [
            "",
            "Resolution",
            "hearing_result",
            if family == "resolution" {
                "notification"
            } else {
                "resolution"
            },
        ] {
            inconsistent(codec::values(wrong, &canonical, projection));
        }
    }
}

#[test]
fn sources_reject_truncated_trailing_and_damaged_canonical_bytes() {
    for vector in source_vectors() {
        let canonical = bytes(&vector);
        for length in [0, 5, canonical.len() / 2, canonical.len() - 1] {
            inconsistent(codec::sources(&canonical[..length], &vector["sources"]));
        }
        let mut changed = canonical.clone();
        changed.push(0);
        inconsistent(codec::sources(&changed, &vector["sources"]));
        changed.pop();
        changed[0] ^= 1;
        inconsistent(codec::sources(&changed, &vector["sources"]));
    }
}

#[test]
fn value_text_is_never_silently_normalized_or_truncated() {
    reject_changed_values(
        "resolution_minimum",
        "/summary",
        vec![
            json!(" x "),
            json!(""),
            json!("x\u{0000}"),
            json!(42),
            json!(null),
            json!("x".repeat(1001)),
            json!("\u{1f642}".repeat(1001)),
        ],
    );
    reject_changed_values(
        "resolution_hearing_agreement_support_normalized",
        "/summary",
        vec![
            json!("Summary\r\nSecond line"),
            json!("Summary\rSecond line"),
        ],
    );
    reject_changed_values(
        "resolution_minimum",
        "/issuer/value",
        vec![json!(" x "), json!("x".repeat(201)), json!(false)],
    );
    reject_changed_values(
        "notification_shared_support_distinct_locators",
        "/provenance/support/locator",
        vec![json!(" p. 1 "), json!("x".repeat(201))],
    );
}

#[test]
fn values_reject_unknown_tags_bad_types_and_out_of_range_counters() {
    for path in [
        "/character/value/kind",
        "/medium/value/kind",
        "/context/value/kind",
        "/outcome/value/kind",
    ] {
        reject_changed_values(
            "notification_minimum",
            path,
            vec![json!("other_unknown"), json!(0)],
        );
    }
    reject_changed_values(
        "notification_minimum",
        "/resolution/revision",
        vec![
            json!(0),
            json!(-1),
            json!(4294967296u64),
            json!(1.5),
            json!("1"),
            json!(true),
        ],
    );
    reject_changed_values(
        "resolution_minimum",
        "/class/kind",
        vec![json!("Known"), json!(null)],
    );
    reject_changed_values(
        "notification_shared_support_distinct_locators",
        "/representation/kind",
        vec![json!("unknown")],
    );
    reject_changed_values(
        "notification_shared_support_distinct_locators",
        "/intended_recipient/value/revision",
        vec![json!(0)],
    );
}

#[test]
fn values_preserve_time_precision_and_reject_impossible_or_unencoded_components() {
    for (path, changes) in [
        (
            "/issued_at/year",
            vec![json!(0), json!(10000), json!(2026.5)],
        ),
        ("/issued_at/month", vec![json!(0), json!(13), json!(2)]),
        ("/issued_at/day", vec![json!(0), json!(32)]),
        (
            "/issued_at/offset_seconds",
            vec![json!(1), json!(50460), json!("0")],
        ),
    ] {
        reject_changed_values("resolution_date_offset_0", path, changes);
    }
    reject_changed_values(
        "resolution_minute_offset_0",
        "/issued_at/hour",
        vec![json!(24), json!(-1)],
    );
    reject_changed_values(
        "resolution_second_offset_0",
        "/issued_at/second",
        vec![json!(60), json!(1.2)],
    );
    let vector = value_vector("resolution_date_offset_0");
    let mut projection = vector["normalized"].clone();
    projection["issued_at"]["hour"] = json!(0);
    reject_value(&vector, &projection);
    let mut projection = vector["normalized"].clone();
    projection["issued_at"]["year"] = json!(2025);
    projection["issued_at"]["month"] = json!(2);
    projection["issued_at"]["day"] = json!(29);
    reject_value(&vector, &projection);
}

#[test]
fn noncanonical_uuid_and_digest_spelling_is_rejected() {
    let name = "notification_shared_support_distinct_locators";
    reject_changed_values(
        name,
        "/provenance/support/document_id",
        vec![
            json!("00000000000000000000000000000001"),
            json!("urn:uuid:00000000-0000-0000-0000-000000000001"),
            json!(42),
        ],
    );
    let vector = value_vector(name);
    let digest = vector["normalized"]["provenance"]["support"]["digest"]
        .as_str()
        .unwrap();
    reject_changed_values(
        name,
        "/provenance/support/digest",
        vec![json!(digest.to_uppercase()), json!("00"), json!(false)],
    );
    reject_changed_sources(
        "complete_mixed",
        "/direct_supports/0/id",
        vec![json!("00000000000000000000000000000000")],
    );
}

#[test]
fn sources_reject_reordering_duplicates_and_excess_cardinality() {
    let vector = source_vector("complete_mixed");
    for field in ["participants", "hearing_results", "direct_supports"] {
        let mut projection = vector["sources"].clone();
        projection[field].as_array_mut().unwrap().reverse();
        reject_source(&vector, &projection);
        let mut projection = vector["sources"].clone();
        let list = projection[field].as_array_mut().unwrap();
        list.push(list[0].clone());
        reject_source(&vector, &projection);
        let mut projection = vector["sources"].clone();
        projection[field] = json!(null);
        reject_source(&vector, &projection);
    }
}

#[test]
fn sources_reject_conflicting_common_result_and_agreement_presence() {
    reject_changed_sources(
        "complete_mixed",
        "/hearing_results/1/summary",
        vec![json!("Other session")],
    );
    reject_changed_sources(
        "complete_mixed",
        "/hearing_results/1/agreement_text",
        vec![json!(null)],
    );
    reject_changed_sources(
        "complete_mixed",
        "/hearing_results/0/agreement_text",
        vec![json!("Undeclared agreement")],
    );
    reject_changed_sources(
        "complete_mixed",
        "/hearing_results/1/status",
        vec![json!("withdrawn"), json!("unknown")],
    );
    reject_changed_sources(
        "complete_mixed",
        "/hearing_results/0/occurrence",
        vec![json!("unknown")],
    );
}

#[test]
fn sources_reject_noncanonical_text_subject_shapes_and_document_policies() {
    reject_changed_sources(
        "complete_mixed",
        "/participants/0/display_name",
        vec![json!(" Name e\u{301} \u{e9} "), json!(false)],
    );
    reject_changed_sources(
        "complete_mixed",
        "/participants/0/organization",
        vec![json!(""), json!(42)],
    );
    reject_changed_sources(
        "complete_mixed",
        "/participants/1/kind",
        vec![json!(null), json!("invalid")],
    );
    reject_changed_sources(
        "complete_mixed",
        "/participants/1/subject/revision",
        vec![json!(0), json!(4294967296u64)],
    );
    reject_changed_sources(
        "complete_mixed",
        "/direct_supports/0/name",
        vec![json!("../support.pdf"), json!(" "), json!("x".repeat(129))],
    );
    reject_changed_sources(
        "complete_mixed",
        "/direct_supports/0/policy",
        vec![json!("other"), json!(0)],
    );
    reject_changed_sources(
        "complete_mixed",
        "/direct_supports/0/format",
        vec![json!("PDF"), json!(null)],
    );
}

#[test]
fn sources_reject_wrong_time_shapes_without_inventing_components() {
    reject_changed_sources(
        "historical_hearing_instant",
        "/hearing_results/0/event_time/hour",
        vec![json!(24), json!(0.5)],
    );
    reject_changed_sources(
        "historical_hearing_instant",
        "/hearing_results/0/event_time/offset_seconds",
        vec![json!(1), json!(50460)],
    );
    reject_changed_sources(
        "historical_hearing_instant",
        "/hearing_results/0/event_time/date",
        vec![json!("2027-02-29"), json!("2028-2-29")],
    );
    let vector = source_vector("resolution_date_0");
    let mut changed = vector["sources"].clone();
    changed["resolution"]["issued_at"]["hour"] = json!(0);
    reject_source(&vector, &changed);
}
