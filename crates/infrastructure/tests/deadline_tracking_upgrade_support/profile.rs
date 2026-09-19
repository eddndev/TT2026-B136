use super::Fixture;
use application::deadline_profiles::*;
use domain::crypto::DocumentHasher;
use infrastructure::RingSha256Hasher;

#[allow(dead_code)]
#[path = "../../../application/tests/deadline_profile_catalog_support/values.rs"]
mod values;

pub fn seed(db: &mut Fixture) -> DeadlineProfileDetail {
    let definition = values::definition(Some(db.case));
    let command = DeadlineProfileCommand {
        operation_id: DeadlineProfileOperationId::new(),
        profile_id: DeadlineProfileId::new(),
        change: DeadlineProfileChange::Publish {
            definition: definition.clone(),
        },
    };
    let canonical = deadline_profile_definition_bytes(&definition);
    let definition_digest = RingSha256Hasher.hash_bytes(&canonical);
    let submission = deadline_profile_submission_bytes(
        db.owner,
        &command,
        DeadlineProfileAlgorithm::V1,
        definition_digest,
    );
    let submission_digest = RingSha256Hasher.hash_bytes(&submission);
    let detail = DeadlineProfileDetail {
        id: command.profile_id,
        revision: DeadlineProfileRevision::initial(),
        definition,
        definition_digest,
        algorithm: DeadlineProfileAlgorithm::V1,
        status: DeadlineProfileStatus::Published,
        reason: None,
        receipt: DeadlineProfileReceipt {
            operation_id: command.operation_id,
            action: DeadlineProfileAction::Publish,
            expected_revision: 0,
            submission_digest,
        },
        recorded_at: db.at,
        recorded_by: DeadlineProfileActorSnapshot {
            id: db.owner,
            email: "owner@example.test".into(),
        },
    };
    deadline_profile_receipt_matches(&RingSha256Hasher, &detail).unwrap();
    let mut tx = db.admin.transaction().unwrap();
    tx.execute(
        "INSERT INTO deadline_profiles(id,case_id) VALUES($1,$2)",
        &[&detail.id.as_uuid(), &db.case.as_uuid()],
    )
    .unwrap();
    tx.execute(
        "INSERT INTO deadline_profile_revisions(
        profile_id,revision,definition_canonical,definition_digest,algorithm,operation_id,
        action,reason,submission_canonical,submission_digest,recorded_at_seconds,
        recorded_at_nanoseconds,recorded_by,recorded_by_email)
        VALUES($1,1,$2,$3,1,$4,'publish',NULL,$5,$6,$7,$8,$9,$10)",
        &[
            &detail.id.as_uuid(),
            &canonical,
            &definition_digest.as_bytes().as_slice(),
            &command.operation_id.as_uuid(),
            &submission,
            &submission_digest.as_bytes().as_slice(),
            &db.at.unix_timestamp(),
            &(db.at.nanosecond() as i32),
            &db.owner.as_uuid(),
            &detail.recorded_by.email,
        ],
    )
    .unwrap();
    tx.commit().unwrap();
    detail
}
