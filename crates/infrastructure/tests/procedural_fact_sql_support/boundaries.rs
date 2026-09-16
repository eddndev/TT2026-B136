use super::*;

fn text(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u32).to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}
fn declared_time(
    precision: u8,
    date: (u16, u8, u8),
    time: [u8; 3],
    offset: Option<i32>,
) -> Vec<u8> {
    let mut out = vec![precision];
    out.extend_from_slice(&date.0.to_be_bytes());
    out.extend([date.1, date.2]);
    if precision >= 2 {
        out.extend_from_slice(&time[..2]);
    }
    if precision == 3 {
        out.push(time[2]);
    }
    out.push(u8::from(offset.is_some()));
    if let Some(offset) = offset {
        out.extend_from_slice(&offset.to_be_bytes());
    }
    out
}
fn resolution_time(time: Vec<u8>) -> Vec<u8> {
    let original = bytes(fixture("resolution_minimum")["hex"].as_str().unwrap());
    let mut out = original[..15].to_vec();
    out.extend(time);
    out.extend_from_slice(&original[16..]);
    out
}
fn sources(participants: &[Vec<u8>], hearings: &[Vec<u8>], supports: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"PFSRC1\0".to_vec();
    for list in [participants, hearings, supports] {
        out.extend_from_slice(&(list.len() as u32).to_be_bytes());
        for item in list {
            out.extend(item);
        }
    }
    out
}
fn participant(id: u8, revision: u32, typed_kind: bool) -> Vec<u8> {
    let mut out = vec![0; 16];
    out.extend([id; 16]);
    out.extend(revision.to_be_bytes());
    out.extend([1; 32]);
    out.extend([0, 0]); // Active and no bound subject.
    text(&mut out, "Person");
    text(&mut out, "defendant");
    out.extend([0, u8::from(typed_kind)]);
    if typed_kind {
        out.push(0);
    }
    out
}
fn hearing(agreement: bool) -> Vec<u8> {
    let mut out = vec![0; 48]; // Case, hearing and result UUIDs.
    out.extend(1u32.to_be_bytes());
    out.push(u8::from(agreement));
    if agreement {
        out.extend([0; 16]);
    }
    out.extend([1; 32]);
    out.extend([2; 32]);
    out.extend([0, 0, 0]); // Recorded, occurred and date precision.
    out.extend(2026u16.to_be_bytes());
    out.extend([9, 16]);
    out.extend(0i32.to_be_bytes());
    text(&mut out, "Declared result");
    out.push(u8::from(agreement));
    if agreement {
        text(&mut out, "Agreement");
    }
    out
}
fn support(name: &str) -> Vec<u8> {
    let mut out = vec![0; 16];
    out.extend(1u32.to_be_bytes());
    out.extend([1; 32]);
    text(&mut out, name);
    out.extend([0, 0]); // PDF and the admitted PDF/DOCX policy.
    out
}
#[test]
fn time_checks_complete_utc_intervals_without_inventing_missing_components() {
    let Some(mut db) = Fixture::new() else { return };
    for time in [
        declared_time(1, (1, 1, 1), [0, 0, 0], Some(60)),
        declared_time(1, (9999, 12, 31), [0, 0, 0], Some(-60)),
        declared_time(2, (1, 1, 1), [0, 0, 0], Some(60)),
        declared_time(2, (9999, 12, 31), [23, 59, 0], Some(-60)),
        declared_time(3, (2026, 2, 29), [0, 0, 0], None),
        declared_time(3, (2026, 1, 1), [0, 0, 60], None),
        declared_time(3, (2026, 1, 1), [0, 0, 0], Some(1)),
        declared_time(3, (2026, 1, 1), [0, 0, 0], Some(50460)),
        declared_time(1, (0, 1, 1), [0, 0, 0], None),
        declared_time(1, (10000, 1, 1), [0, 0, 0], None),
    ] {
        rejected(
            &mut db.admin,
            "procedural_fact_values",
            Some("resolution"),
            &resolution_time(time),
        );
    }
    for (time, precision) in [
        (declared_time(1, (1, 1, 1), [0, 0, 0], None), "date"),
        (declared_time(1, (9999, 12, 31), [0, 0, 0], None), "date"),
        (declared_time(2, (1, 1, 1), [0, 1, 0], Some(60)), "minute"),
        (
            declared_time(2, (9999, 12, 31), [23, 58, 0], Some(-60)),
            "minute",
        ),
        (
            declared_time(3, (9999, 12, 31), [23, 59, 59], Some(0)),
            "second",
        ),
    ] {
        let projection = value(&mut db.admin, "resolution", &resolution_time(time));
        assert_eq!(projection["issued_at"]["precision"], precision);
        assert_eq!(
            projection["issued_at"].get("second").is_some(),
            precision == "second"
        );
        assert_eq!(
            projection["issued_at"].get("hour").is_some(),
            precision != "date"
        );
    }
}
#[test]
fn sources_require_strict_keys_and_kind_subject_consistency() {
    let Some(mut db) = Fixture::new() else { return };
    for participants in [
        vec![participant(0, 1, true)],
        vec![participant(0, 0, false)],
        vec![participant(0, 1, false), participant(0, 1, false)],
        vec![participant(1, 1, false), participant(0, 1, false)],
        vec![participant(0, 2, false), participant(0, 1, false)],
    ] {
        rejected(
            &mut db.admin,
            "procedural_fact_sources",
            None,
            &sources(&participants, &[], &[]),
        );
    }
    let valid = sources(
        &[participant(0, 1, false), participant(0, 2, false)],
        &[],
        &[],
    );
    assert!(db
        .admin
        .query_one("SELECT procedural_fact_sources($1)", &[&valid])
        .is_ok());
}
#[test]
fn result_selections_distinguish_none_from_nil_but_share_one_snapshot() {
    let Some(mut db) = Fixture::new() else { return };
    let plain = hearing(false);
    let agreement = hearing(true);
    let valid = sources(&[], &[plain.clone(), agreement.clone()], &[]);
    let projection: Value = db
        .admin
        .query_one("SELECT procedural_fact_sources($1)", &[&valid])
        .unwrap()
        .get(0);
    assert!(projection["hearing_results"][0]["agreement_id"].is_null());
    assert_eq!(
        projection["hearing_results"][1]["agreement_id"],
        "00000000-0000-0000-0000-000000000000"
    );
    for position in [0, 69, 101, 133, 134, 139, 148] {
        let mut changed = agreement.clone();
        changed[position] ^= 1;
        rejected(
            &mut db.admin,
            "procedural_fact_sources",
            None,
            &sources(&[], &[plain.clone(), changed], &[]),
        );
    }
    for pair in [
        vec![agreement.clone(), plain.clone()],
        vec![plain.clone(), plain],
    ] {
        rejected(
            &mut db.admin,
            "procedural_fact_sources",
            None,
            &sources(&[], &pair, &[]),
        );
    }
}
#[test]
fn direct_supports_reject_unsafe_names_invalid_policy_and_duplicate_version() {
    let Some(mut db) = Fixture::new() else { return };
    for name in ["", ".pdf", "../a.pdf", "a/b.pdf", "a\\b.pdf", "a b.pdf"] {
        rejected(
            &mut db.admin,
            "procedural_fact_sources",
            None,
            &sources(&[], &[], &[support(name)]),
        );
    }
    let mut changed = support("a.pdf");
    *changed.last_mut().unwrap() = 1;
    rejected(
        &mut db.admin,
        "procedural_fact_sources",
        None,
        &sources(&[], &[], &[changed]),
    );
    rejected(
        &mut db.admin,
        "procedural_fact_sources",
        None,
        &sources(&[], &[], &[support("a.pdf"), support("b.pdf")]),
    );
}
#[test]
fn duplicated_document_versions_cannot_claim_distinct_support_digests() {
    let Some(mut db) = Fixture::new() else { return };
    let vector = fixture("notification_shared_support_distinct_locators");
    let mut canonical = bytes(vector["hex"].as_str().unwrap());
    let digest = bytes(
        vector["normalized"]["provenance"]["support"]["digest"]
            .as_str()
            .unwrap(),
    );
    let positions: Vec<_> = canonical
        .windows(32)
        .enumerate()
        .filter_map(|(i, window)| (window == digest).then_some(i))
        .collect();
    assert_eq!(positions.len(), 2);
    canonical[positions[1]] ^= 1;
    rejected(
        &mut db.admin,
        "procedural_fact_values",
        Some("notification"),
        &canonical,
    );
}
