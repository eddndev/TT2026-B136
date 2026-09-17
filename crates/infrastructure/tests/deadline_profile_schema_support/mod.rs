#![allow(dead_code)]
use crate::case_administration_support::Fixture;
use domain::{cases::CaseId, crypto::DocumentHasher};
use infrastructure::{PostgresCaseRepository, RingSha256Hasher};
use std::sync::Arc;
use uuid::Uuid;
#[path = "../../../application/tests/deadline_profile_catalog_support/values.rs"]
mod values;

pub fn definition(case: Option<CaseId>) -> Vec<u8> {
    application::deadline_profiles::deadline_profile_definition_bytes(&values::definition(case))
}
pub fn open(db: &Fixture) -> Result<PostgresCaseRepository, application::ApplicationError> {
    PostgresCaseRepository::open(&db.runtime_url, Arc::new(RingSha256Hasher))
}
pub fn receipt(
    actor: Uuid,
    operation: Uuid,
    id: Uuid,
    action: u8,
    expected: u32,
    digest: &[u8],
    reason: Option<&str>,
) -> Vec<u8> {
    let mut bytes = b"DPTX1".to_vec();
    for value in [actor, operation, id] {
        bytes.extend_from_slice(value.as_bytes());
    }
    bytes.push(action);
    bytes.extend_from_slice(&expected.to_be_bytes());
    bytes.push(1);
    bytes.extend_from_slice(digest);
    bytes.push(u8::from(reason.is_some()));
    if let Some(reason) = reason {
        bytes.extend_from_slice(&(reason.len() as u32).to_be_bytes());
        bytes.extend_from_slice(reason.as_bytes());
    }
    bytes
}
pub fn append(
    db: &mut Fixture,
    id: Uuid,
    case: Option<CaseId>,
    revision: u32,
    action: &str,
    definition: &[u8],
) -> Result<(), postgres::Error> {
    let digest = RingSha256Hasher.hash_bytes(definition).as_bytes().to_vec();
    let operation = Uuid::new_v4();
    let actor = db.owner.as_uuid();
    let reason = if action == "publish" {
        None
    } else {
        Some("Stated reason")
    };
    let tag = match action {
        "publish" => 0,
        "replace" => 1,
        "retire" => 2,
        _ => 255,
    };
    let submission = receipt(actor, operation, id, tag, revision - 1, &digest, reason);
    let submission_digest = RingSha256Hasher.hash_bytes(&submission).as_bytes().to_vec();
    let at = db.at;
    let case = case.map(|id| id.as_uuid());
    let email: String = db
        .admin
        .query_one("SELECT email FROM users WHERE id=$1", &[&actor])?
        .get(0);
    let mut tx = db.admin.transaction()?;
    if revision == 1 {
        tx.execute(
            "INSERT INTO deadline_profiles(id,case_id) VALUES($1,$2)",
            &[&id, &case],
        )?;
    }
    tx.execute("INSERT INTO deadline_profile_revisions(profile_id,revision,definition_canonical,definition_digest,algorithm,operation_id,action,reason,submission_canonical,submission_digest,recorded_at_seconds,recorded_at_nanoseconds,recorded_by,recorded_by_email) VALUES($1,$2,$3,$4,1,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
        &[&id,&i64::from(revision),&definition,&digest,&operation,&action,&reason,&submission,&submission_digest,&at.unix_timestamp(),&(at.nanosecond() as i32),&actor,&email])?;
    tx.commit()
}
pub fn altered_global_definition() -> Vec<u8> {
    let mut input = values::input(None);
    input.scope = values::altered_global_scope();
    application::deadline_profiles::deadline_profile_definition_bytes(
        &application::deadline_profiles::DeadlineProfileDefinition::new(input).unwrap(),
    )
}
