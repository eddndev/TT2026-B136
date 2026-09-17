mod case_administration_support;
mod procedural_fact_sql_support;
use case_administration_support::Fixture;
use procedural_fact_sql_support::*;
use serde_json::Value;

#[test]
fn sql_values_match_every_independent_normalized_domain_vector() {
    let Some(mut db) = Fixture::new() else { return };
    for vector in values() {
        let canonical = bytes(vector["hex"].as_str().unwrap());
        assert_eq!(
            value(
                &mut db.admin,
                vector["family"].as_str().unwrap(),
                &canonical
            ),
            vector["normalized"],
            "{}",
            vector["name"]
        );
    }
}
#[test]
fn sql_sources_match_every_independent_source_projection() {
    let Some(mut db) = Fixture::new() else { return };
    for vector in receipts()["sources"].as_array().unwrap() {
        let canonical = bytes(vector["hex"].as_str().unwrap());
        let projection: Value = db
            .admin
            .query_one("SELECT procedural_fact_sources($1)", &[&canonical])
            .unwrap()
            .get(0);
        assert_eq!(projection, vector["sources"], "{}", vector["name"]);
    }
}
#[test]
fn sql_submissions_match_every_independent_operation_projection() {
    let Some(mut db) = Fixture::new() else { return };
    for vector in receipts()["submissions"].as_array().unwrap() {
        let canonical = bytes(vector["hex"].as_str().unwrap());
        let projection: Value = db
            .admin
            .query_one("SELECT procedural_fact_submission($1)", &[&canonical])
            .unwrap()
            .get(0);
        assert_eq!(projection, vector["submission"], "{}", vector["name"]);
    }
}
#[test]
fn all_formats_reject_truncation_trailing_bytes_wrong_headers_and_null() {
    let Some(mut db) = Fixture::new() else { return };
    let mut cases: Vec<_> = values()
        .into_iter()
        .map(|v| {
            (
                "procedural_fact_values",
                Some(v["family"].as_str().unwrap().to_owned()),
                bytes(v["hex"].as_str().unwrap()),
            )
        })
        .collect();
    let corpus = receipts();
    for (key, function) in [
        ("sources", "procedural_fact_sources"),
        ("submissions", "procedural_fact_submission"),
    ] {
        for v in corpus[key].as_array().unwrap() {
            cases.push((function, None, bytes(v["hex"].as_str().unwrap())));
        }
    }
    for (function, family, canonical) in cases {
        for end in [0, 5, canonical.len() / 2, canonical.len() - 1] {
            rejected(
                &mut db.admin,
                function,
                family.as_deref(),
                &canonical[..end],
            );
        }
        let mut trailing = canonical.clone();
        trailing.push(0);
        rejected(&mut db.admin, function, family.as_deref(), &trailing);
        let mut header = canonical;
        header[0] ^= 1;
        rejected(&mut db.admin, function, family.as_deref(), &header);
    }
    for sql in [
        "SELECT procedural_fact_values(NULL,NULL)",
        "SELECT procedural_fact_sources(NULL)",
        "SELECT procedural_fact_submission(NULL)",
    ] {
        let error = db.admin.query_one(sql, &[]).unwrap_err();
        assert_eq!(error.code().map(|code| code.code()), Some("23514"));
    }
}
#[test]
fn values_reject_wrong_family_unknown_discriminators_and_zero_references() {
    let Some(mut db) = Fixture::new() else { return };
    let res = bytes(fixture("resolution_minimum")["hex"].as_str().unwrap());
    for family in ["notification", "", "Resolution"] {
        rejected(&mut db.admin, "procedural_fact_values", Some(family), &res);
    }
    for position in [6, 7, 8, 9, 15, 21] {
        let mut changed = res.clone();
        changed[position] = 255;
        rejected(
            &mut db.admin,
            "procedural_fact_values",
            Some("resolution"),
            &changed,
        );
    }
    let notice = bytes(fixture("notification_minimum")["hex"].as_str().unwrap());
    for position in [
        26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 44, 50, 61,
    ] {
        let mut changed = notice.clone();
        changed[position] = 255;
        rejected(
            &mut db.admin,
            "procedural_fact_values",
            Some("notification"),
            &changed,
        );
    }
    let mut changed = notice;
    changed[22..26].fill(0);
    rejected(
        &mut db.admin,
        "procedural_fact_values",
        Some("notification"),
        &changed,
    );
}
#[test]
fn canonical_text_rejects_controls_padding_invalid_utf8_and_overlong_scalars() {
    let Some(mut db) = Fixture::new() else { return };
    let canonical = bytes(fixture("resolution_minimum")["hex"].as_str().unwrap());
    for replacement in [
        vec![0],
        vec![9],
        vec![10],
        vec![13],
        vec![32],
        vec![0xc0, 0x80],
        vec![b'x'; 1001],
        b" x".to_vec(),
        b"x ".to_vec(),
        b"x\r\ny".to_vec(),
    ] {
        let mut changed = canonical[..16].to_vec();
        changed.extend_from_slice(&(replacement.len() as u32).to_be_bytes());
        changed.extend(replacement);
        changed.extend_from_slice(&canonical[21..]);
        rejected(
            &mut db.admin,
            "procedural_fact_values",
            Some("resolution"),
            &changed,
        );
    }
}
#[test]
fn sources_reject_non_boolean_options_and_counts_above_fixed_limits() {
    let Some(mut db) = Fixture::new() else { return };
    let canonical = bytes(receipts()["sources"][0]["hex"].as_str().unwrap());
    let mut changed = canonical.clone();
    changed[6] = 2;
    rejected(&mut db.admin, "procedural_fact_sources", None, &changed);
    for (position, count) in [(7, 5u32), (11, 3u32), (15, 3u32), (7, u32::MAX)] {
        let mut changed = canonical.clone();
        changed[position..position + 4].copy_from_slice(&count.to_be_bytes());
        rejected(&mut db.admin, "procedural_fact_sources", None, &changed);
    }
}
#[test]
fn submissions_reject_exhaustion_action_reason_mismatch_and_invalid_family() {
    let Some(mut db) = Fixture::new() else { return };
    let corpus = receipts();
    for vector in corpus["submissions"].as_array().unwrap() {
        let canonical = bytes(vector["hex"].as_str().unwrap());
        let action_position = if vector["submission"]["family"] == "resolution" {
            71
        } else {
            87
        };
        let mut exhausted = canonical.clone();
        exhausted[action_position + 1..action_position + 5].fill(255);
        rejected(
            &mut db.admin,
            "procedural_fact_submission",
            None,
            &exhausted,
        );
        let mut action = canonical.clone();
        action[action_position] = 255;
        rejected(&mut db.admin, "procedural_fact_submission", None, &action);
        let mut family = canonical.clone();
        family[54] = 2;
        rejected(&mut db.admin, "procedural_fact_submission", None, &family);
        let mut reason = canonical;
        reason[action_position + 69] = 2;
        rejected(&mut db.admin, "procedural_fact_submission", None, &reason);
    }
}

#[path = "procedural_fact_sql_support/boundaries.rs"]
mod boundaries;
