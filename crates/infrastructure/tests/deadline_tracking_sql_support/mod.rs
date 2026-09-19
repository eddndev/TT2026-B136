#![allow(dead_code)]
use crate::{
    case_administration_support::Fixture, deadline_observations_sql_support as observations,
};
use application::deadline_reevaluation::{encode_observations, Observations};
use domain::crypto::DocumentHasher;
use infrastructure::RingSha256Hasher;
use serde_json::{json, Value};

/// Independent framing of the unprefixed suffix appended to DLST2.
pub fn frame(
    policies: [u8; 3],
    state: u8,
    reasons: &[[u8; 2]],
    observations_digest: [u8; 32],
    administration: Option<u32>,
) -> Vec<u8> {
    let mut bytes = policies.to_vec();
    bytes.extend([state, reasons.len() as u8]);
    for reason in reasons {
        bytes.extend(reason);
    }
    bytes.extend(observations_digest);
    bytes.push(u8::from(administration.is_some()));
    if let Some(revision) = administration {
        bytes.extend(revision.to_be_bytes());
    }
    bytes.extend([0x22; 32]);
    bytes.extend([0x33; 32]);
    bytes
}

pub fn expected(
    policies: [u8; 3],
    state: u8,
    reasons: &[[u8; 2]],
    observations_digest: [u8; 32],
    administration: Option<u32>,
) -> Value {
    let policy = ["undetermined", "fixed", "follow"];
    let states = ["legacy_undeclared", "accepted", "pending"];
    let dependency = ["profile", "source", "calendar"];
    let names = [
        "source_changed",
        "profile_changed",
        "dependency_retired",
        "policy_undetermined",
    ];
    let reasons: Vec<_> = reasons
        .iter()
        .map(|[scope, reason]| {
            json!({
                "dependency": dependency[usize::from(*scope)], "reason": names[usize::from(*reason)]
            })
        })
        .collect();
    let hex: String = observations_digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    json!({
        "policies": {"profile":policy[usize::from(policies[0])],
            "source":policy[usize::from(policies[1])],"calendar":policy[usize::from(policies[2])]},
        "review": {"state":states[usize::from(state)],"reasons":reasons},
        "observations_digest":hex,"administration_revision":administration,
        "administration_values_digest":"22".repeat(32),
        "administration_evidence_digest":"33".repeat(32)
    })
}

pub fn parse(db: &mut Fixture, bytes: &[u8]) -> Value {
    db.admin
        .query_one("SELECT deadline_tracking($1::bytea)", &[&bytes])
        .unwrap()
        .get(0)
}

pub fn fixture() -> Option<Fixture> {
    let mut db = observations::fixture()?;
    let bytes = frame([0; 3], 0, &[], [0x11; 32], None);
    assert_eq!(
        parse(&mut db, &bytes),
        expected([0; 3], 0, &[], [0x11; 32], None)
    );
    Some(db)
}

pub fn rejected(db: &mut Fixture, bytes: &[u8]) {
    let error = db
        .admin
        .query_one("SELECT deadline_tracking($1::bytea)", &[&bytes])
        .expect_err("PostgreSQL accepted invalid tracking bytes");
    assert_eq!(
        error.code(),
        Some(&postgres::error::SqlState::CHECK_VIOLATION)
    );
}

pub fn pair(
    value: &Observations,
    policies: [u8; 3],
    state: u8,
    reasons: &[[u8; 2]],
) -> (Vec<u8>, Vec<u8>) {
    let manifest = encode_observations(value).unwrap();
    let digest = RingSha256Hasher.hash_bytes(&manifest);
    (
        frame(policies, state, reasons, *digest.as_bytes(), Some(1)),
        manifest,
    )
}

pub fn consistent(db: &mut Fixture, tracking: Option<&[u8]>, manifest: Option<&[u8]>) -> bool {
    db.admin
        .query_one(
            "SELECT deadline_tracking_consistent($1::bytea,$2::bytea)",
            &[&tracking, &manifest],
        )
        .unwrap()
        .get(0)
}

pub fn inconsistent_bytes(db: &mut Fixture, tracking: &[u8], manifest: &[u8]) {
    let error = db
        .admin
        .query_one(
            "SELECT deadline_tracking_consistent($1::bytea,$2::bytea)",
            &[&tracking, &manifest],
        )
        .expect_err("invalid bytes must raise instead of returning false");
    assert_eq!(
        error.code(),
        Some(&postgres::error::SqlState::CHECK_VIOLATION)
    );
}
