mod case_administration_support;
mod deadline_tracking_receipt_support;

use application::deadline_reevaluation::*;
use case_administration_support::Fixture;
use deadline_tracking_receipt_support::*;
use postgres::error::SqlState;
use std::ops::Range;
use uuid::Uuid;

fn reject(db: &mut Fixture, bytes: &[u8], label: &str) {
    assert!(
        decode_tracked_submission(bytes).is_err(),
        "Rust accepted {label}"
    );
    let error = db
        .admin
        .query_one("SELECT deadline_submission($1)", &[&bytes])
        .expect_err(label);
    assert_eq!(
        error.code(),
        Some(&SqlState::CHECK_VIOLATION),
        "{label}: {error}"
    );
}

fn replace(bytes: &[u8], range: Range<usize>, value: &[u8]) -> Vec<u8> {
    let mut changed = bytes.to_vec();
    changed.splice(range, value.iter().copied());
    changed
}

fn text(bytes: &[u8], offset: usize, old_length: usize, value: &[u8]) -> Vec<u8> {
    let mut framed = (value.len() as u64).to_be_bytes().to_vec();
    framed.extend(value);
    replace(bytes, offset..offset + 8 + old_length, &framed)
}

#[test]
fn sql_rejects_every_truncation_and_trailing_bytes_of_each_receipt_shape() {
    let Some(mut db) = Fixture::new() else { return };
    let bootstrap = TrackedSubmission {
        cause: Some(TechnicalCause::LegacyBootstrap {
            job_id: Uuid::nil(),
            policy_version: 1,
        }),
        ..technical(&db)
    };
    for value in [register(&db), correction(&db), technical(&db), bootstrap] {
        let bytes = assert_projection(&mut db, &value);
        for end in 0..bytes.len() {
            reject(
                &mut db,
                &bytes[..end],
                &format!("truncation {end}/{}", bytes.len()),
            );
        }
        let mut trailing = bytes;
        trailing.push(0);
        reject(&mut db, &trailing, "trailing byte");
    }
}

#[test]
fn sql_rejects_unknown_prefixes_and_does_not_fall_back_to_another_version() {
    let Some(mut db) = Fixture::new() else { return };
    let value = register(&db);
    let bytes = assert_projection(&mut db, &value);
    for prefix in [b"DLTX0", b"DLTX1", b"DLTX3", b"DLRV2", b"dltx2"] {
        reject(
            &mut db,
            &replace(&bytes, 0..5, prefix),
            "unsupported prefix",
        );
    }
    let error = db
        .admin
        .query_one("SELECT deadline_submission(NULL::bytea)", &[])
        .unwrap_err();
    assert_eq!(error.code(), Some(&SqlState::CHECK_VIOLATION));
}

#[test]
fn sql_rejects_unknown_discriminants_in_every_receipt_layer() {
    let Some(mut db) = Fixture::new() else { return };
    let value = register(&db);
    let manual = assert_projection(&mut db, &value);
    for (offset, tag) in [(53, 5), (122, 2), (123, 2), (149, 2), (150, 3)] {
        reject(
            &mut db,
            &replace(&manual, offset..offset + 1, &[tag]),
            "manual tag",
        );
    }
    let value = technical(&db);
    let technical = assert_projection(&mut db, &value);
    for (offset, tag) in [
        (187, 2),
        (188, 1),
        (191, 2),
        (207, 3),
        (232, 5),
        (253, 2),
        (270, 2),
    ] {
        reject(
            &mut db,
            &replace(&technical, offset..offset + 1, &[tag]),
            "technical tag",
        );
    }
}

#[test]
fn sql_requires_action_revision_predecessor_and_reason_to_agree() {
    let Some(mut db) = Fixture::new() else { return };
    let value = register(&db);
    let initial = assert_projection(&mut db, &value);
    let value = correction(&db);
    let later = assert_projection(&mut db, &value);
    reject(
        &mut db,
        &replace(&initial, 54..58, &1_u32.to_be_bytes()),
        "register revision",
    );
    let mut predecessor = vec![1];
    predecessor.extend([0x33; 64]);
    reject(
        &mut db,
        &replace(&initial, 122..123, &predecessor),
        "register predecessor",
    );
    let mut reason = vec![1];
    reason.extend(1_u64.to_be_bytes());
    reason.push(b'x');
    reject(
        &mut db,
        &replace(&initial, 149..150, &reason),
        "register reason",
    );
    for revision in [0, u32::MAX] {
        reject(
            &mut db,
            &replace(&later, 54..58, &revision.to_be_bytes()),
            "followup revision",
        );
    }
    reject(
        &mut db,
        &replace(&later, 122..187, &[0]),
        "missing predecessor",
    );
    reject(&mut db, &replace(&later, 213..229, &[0]), "missing reason");
    reject(
        &mut db,
        &replace(&later, 53..54, &[0]),
        "register with followup fields",
    );
}

#[test]
fn sql_does_not_confuse_human_actions_with_a_technical_author_or_cause() {
    let Some(mut db) = Fixture::new() else { return };
    let value = correction(&db);
    let human = assert_projection(&mut db, &value);
    let value = technical(&db);
    let service = assert_projection(&mut db, &value);
    reject(
        &mut db,
        &replace(&human, 53..54, &[4]),
        "human reevaluation",
    );
    reject(
        &mut db,
        &replace(&human, 229..230, &service[207..]),
        "human technical cause",
    );
    for action in 0..4 {
        reject(
            &mut db,
            &replace(&service, 53..54, &[action]),
            "technical human action",
        );
    }
    reject(
        &mut db,
        &replace(&service, 207..303, &[0]),
        "technical missing cause",
    );
    reject(
        &mut db,
        &replace(&service, 191..207, &[0]),
        "technical missing reason",
    );
    reject(
        &mut db,
        &replace(&service, 122..187, &[0]),
        "technical missing predecessor",
    );
}

#[test]
fn sql_rejects_unsupported_service_and_bootstrap_policy_versions() {
    let Some(mut db) = Fixture::new() else { return };
    let value = technical(&db);
    let service = assert_projection(&mut db, &value);
    let value = TrackedSubmission {
        cause: Some(TechnicalCause::LegacyBootstrap {
            job_id: Uuid::nil(),
            policy_version: 1,
        }),
        ..technical(&db)
    };
    let bootstrap = assert_projection(&mut db, &value);
    for policy in [0_u16, 2, u16::MAX] {
        reject(
            &mut db,
            &replace(&service, 189..191, &policy.to_be_bytes()),
            "service policy",
        );
        reject(
            &mut db,
            &replace(&bootstrap, 224..226, &policy.to_be_bytes()),
            "bootstrap policy",
        );
    }
}

#[test]
fn sql_rejects_source_event_numbers_outside_the_durable_event_domain() {
    let Some(mut db) = Fixture::new() else { return };
    let value = technical(&db);
    let bytes = assert_projection(&mut db, &value);
    for sequence in [0, i64::MAX as u64 + 1, u64::MAX] {
        reject(
            &mut db,
            &replace(&bytes, 224..232, &sequence.to_be_bytes()),
            "event sequence",
        );
    }
    reject(
        &mut db,
        &replace(&bytes, 249..253, &0_u32.to_be_bytes()),
        "event revision",
    );
}

#[test]
fn sql_rejects_cross_case_events_and_family_scope_mismatches() {
    let Some(mut db) = Fixture::new() else { return };
    let value = technical(&db);
    let bytes = assert_projection(&mut db, &value);
    let foreign = Uuid::new_v4();
    assert_ne!(foreign, db.case.as_uuid());
    reject(
        &mut db,
        &replace(&bytes, 254..270, foreign.as_bytes()),
        "foreign case",
    );
    reject(
        &mut db,
        &replace(&bytes, 253..270, &[0]),
        "hearing result without case",
    );
    reject(
        &mut db,
        &replace(&bytes, 270..287, &[0]),
        "hearing result without hearing",
    );
    for family in [0, 1, 3, 4] {
        reject(
            &mut db,
            &replace(&bytes, 232..233, &[family]),
            "unexpected hearing scope",
        );
    }
    let without_hearing = replace(&bytes, 270..287, &[0]);
    for family in [0, 1] {
        let fact = replace(&without_hearing, 232..233, &[family]);
        reject(
            &mut db,
            &replace(&fact, 253..270, &[0]),
            "fact without case",
        );
    }
    let calendar = replace(&without_hearing, 232..233, &[3]);
    reject(&mut db, &calendar, "calendar with case");
    let profile = replace(&without_hearing, 232..233, &[4]);
    reject(
        &mut db,
        &replace(&profile, 254..270, foreign.as_bytes()),
        "foreign profile scope",
    );
}

#[test]
fn sql_rejects_noncanonical_empty_control_and_oversized_email_text() {
    let Some(mut db) = Fixture::new() else { return };
    let value = register(&db);
    let bytes = assert_projection(&mut db, &value);
    for invalid in [
        "",
        " ",
        " a",
        "a ",
        "bad\nemail",
        "a\0b",
        "a\u{7f}",
        "\u{a0}a",
    ] {
        reject(
            &mut db,
            &text(&bytes, 140, 1, invalid.as_bytes()),
            "noncanonical email",
        );
    }
    for invalid in ["x".repeat(321), "\u{1f4c4}".repeat(321)] {
        reject(
            &mut db,
            &text(&bytes, 140, 1, invalid.as_bytes()),
            "email scalar limit",
        );
    }
    reject(
        &mut db,
        &text(&bytes, 140, 1, &[0xff]),
        "invalid email UTF8",
    );
}

#[test]
fn sql_rejects_noncanonical_reason_text_without_normalizing_it() {
    let Some(mut db) = Fixture::new() else { return };
    let value = correction(&db);
    let bytes = assert_projection(&mut db, &value);
    for invalid in [
        "",
        " ",
        " Changed",
        "Changed ",
        "Same\r\ntext",
        "Same\rtext",
        "a\0b",
        "a\u{7f}",
        "a\ttext",
    ] {
        reject(
            &mut db,
            &text(&bytes, 214, 7, invalid.as_bytes()),
            "noncanonical reason",
        );
    }
    for invalid in ["x".repeat(1001), "\u{1f4c4}".repeat(1001)] {
        reject(
            &mut db,
            &text(&bytes, 214, 7, invalid.as_bytes()),
            "reason scalar limit",
        );
    }
    reject(
        &mut db,
        &text(&bytes, 214, 7, &[0xff]),
        "invalid reason UTF8",
    );
}

#[test]
fn sql_rejects_oversized_or_inconsistent_text_length_frames() {
    let Some(mut db) = Fixture::new() else { return };
    let value = register(&db);
    let initial = assert_projection(&mut db, &value);
    let value = correction(&db);
    let later = assert_projection(&mut db, &value);
    for size in [0, 2, 1281, 1_u64 << 32, u64::MAX] {
        reject(
            &mut db,
            &replace(&initial, 140..148, &size.to_be_bytes()),
            "email frame",
        );
    }
    for size in [0, 6, 8, 4001, 1_u64 << 32, u64::MAX] {
        reject(
            &mut db,
            &replace(&later, 214..222, &size.to_be_bytes()),
            "reason frame",
        );
    }
    let mut value = correction(&db);
    value.author = TrackedAuthor::User {
        id: db.owner,
        email: "\u{1f4c4}".repeat(320),
    };
    value.reason = Some("\u{1f4c4}".repeat(1000));
    let largest = assert_projection(&mut db, &value);
    assert_eq!(largest.len(), MAX_TRACKED_SUBMISSION_BYTES);
    let mut over = largest;
    over.push(0);
    reject(&mut db, &over, "total receipt length");
}
