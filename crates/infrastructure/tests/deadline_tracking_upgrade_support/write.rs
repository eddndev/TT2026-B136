use super::Fixture;
use application::{
    deadline_evaluations::{deadline_evaluation_input_bytes, deadline_evaluation_record_bytes},
    deadlines::*,
};
use domain::crypto::DocumentHasher;
use infrastructure::RingSha256Hasher;
use serde_json::{json, Value};

pub fn insert(db: &mut Fixture, detail: &DeadlineDetail, command: &DeadlineCommand) {
    deadline_receipt_matches(&RingSha256Hasher, detail).unwrap();
    assert_eq!(detail.receipt.version, DeadlineReceiptVersion::Legacy);
    assert!(detail.tracking.is_none());
    assert!(detail.definition.input.calendar.is_none());
    assert!(detail.calculation.material.source.is_none());
    let input = deadline_evaluation_input_bytes(&detail.definition.input).unwrap();
    let result = deadline_evaluation_record_bytes(&detail.calculation.result);
    let admin = detail
        .calculation
        .material
        .administration
        .values()
        .canonical_bytes();
    let admin_digest = RingSha256Hasher.hash_bytes(&admin);
    let review = deadline_review_bytes(&RingSha256Hasher, detail).unwrap();
    let capture = deadline_capture_bytes(&RingSha256Hasher, detail).unwrap();
    let submission =
        deadline_submission_bytes(db.owner, db.case, command, detail.receipt.review_digest);
    assert_eq!(&review[..5], b"DLRV1");
    assert_eq!(&capture[..5], b"DLST1");
    assert_eq!(&submission[..5], b"DLTX1");
    let mut tx = db.admin.transaction().unwrap();
    if detail.revision.get() == 1 {
        tx.execute(
            "INSERT INTO case_deadlines(id,case_id) VALUES($1,$2)",
            &[&detail.id.as_uuid(), &db.case.as_uuid()],
        )
        .unwrap();
    }
    tx.execute(
        "INSERT INTO case_deadline_revisions(
        deadline_id,case_id,revision,title,profile_id,profile_revision,input_canonical,
        result_canonical,observed_administration_canonical,observed_administration_digest,
        responsible_id,responsible_email,responsible_role,attention,operation_id,action,reason,
        review_canonical,review_digest,capture_canonical,capture_digest,submission_canonical,
        submission_digest,recorded_at_seconds,recorded_at_nanoseconds,recorded_by,recorded_by_email)
        VALUES($1,$2,$3,$4,$5,1,$6,$7,$8,$9,$10,$11,'owner',$12,$13,$14,$15,
        $16,$17,$18,$19,$20,$21,$22,$23,$24,$25)",
        &[
            &detail.id.as_uuid(),
            &db.case.as_uuid(),
            &i64::from(detail.revision.get()),
            &detail.definition.title.as_str(),
            &detail.definition.profile.id.as_uuid(),
            &input,
            &result,
            &admin,
            &admin_digest.as_bytes().as_slice(),
            &detail.responsible.id.as_uuid(),
            &detail.responsible.email,
            &attention(&detail.attention),
            &detail.receipt.operation_id.as_uuid(),
            &detail.receipt.action.as_str(),
            &detail.reason.as_ref().map(|value| value.as_str()),
            &review,
            &detail.receipt.review_digest.as_bytes().as_slice(),
            &capture,
            &detail.receipt.capture_digest.as_bytes().as_slice(),
            &submission,
            &detail.receipt.submission_digest.as_bytes().as_slice(),
            &detail.recorded_at.unix_timestamp(),
            &(detail.recorded_at.nanosecond() as i32),
            &db.owner.as_uuid(),
            &"owner@example.test",
        ],
    )
    .unwrap();
    tx.commit().unwrap();
}

fn attention(value: &DeadlineAttention) -> Value {
    match value {
        DeadlineAttention::Pending => json!({"status":"pending"}),
        DeadlineAttention::Recorded {
            occurred_at,
            statement,
            locator,
        } => {
            assert_eq!(
                *occurred_at,
                domain::procedural_time::DeclaredProceduralTime::unknown()
            );
            json!({"status":"recorded","occurred_at":{"precision":"unknown"},
                "statement":statement.as_str(),"locator":locator.as_str()})
        }
    }
}
