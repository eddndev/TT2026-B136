#![allow(dead_code)]
use crate::deadline_backend_support::Fixture;
use application::deadlines::*;
use domain::{
    crypto::DocumentHasher,
    procedural_facts::{FactLabel, FactText},
};
use infrastructure::RingSha256Hasher;
use serde_json::Value;

pub fn attention(
    base: &DeadlineDetail,
    at: domain::procedural_time::DeclaredProceduralTime,
) -> DeadlineCommand {
    DeadlineCommand {
        operation_id: DeadlineOperationId::new(),
        deadline_id: base.id,
        change: DeadlineChange::SetAttention {
            expected_revision: base.revision,
            attention: DeadlineAttention::Recorded {
                occurred_at: at,
                statement: FactText::new("Declared action").unwrap(),
                locator: FactLabel::new("Captured record").unwrap(),
            },
            reason: FactText::new("Record declared attention").unwrap(),
        },
    }
}

/// Simulate damaged storage after a runtime store has already opened successfully.
pub fn remove_checks(db: &mut Fixture) {
    db.admin
        .batch_execute(
            "ALTER TABLE case_deadline_revisions DISABLE TRIGGER USER;
        DO $$ DECLARE c record; BEGIN
        FOR c IN SELECT conname FROM pg_constraint
          WHERE conrelid='case_deadline_revisions'::regclass AND contype='c'
        LOOP EXECUTE format('ALTER TABLE case_deadline_revisions DROP CONSTRAINT %I',c.conname);
        END LOOP; END $$;",
        )
        .unwrap();
}
pub fn audit(db: &mut Fixture) -> Value {
    db.admin.query_one("SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY sequence),'[]'::jsonb) FROM audit_events a", &[]).unwrap().get(0)
}
pub fn set_attention(db: &mut Fixture, detail: &DeadlineDetail, value: &Value) {
    db.admin
        .execute(
            "UPDATE case_deadline_revisions SET attention=$3
        WHERE deadline_id=$1 AND revision=$2",
            &[
                &detail.id.as_uuid(),
                &i64::from(detail.revision.get()),
                value,
            ],
        )
        .unwrap();
}
pub fn change_title(base: &DeadlineDetail) -> DeadlineCommand {
    let mut definition = base.definition.clone();
    definition.title = FactLabel::new("Corrected title").unwrap();
    DeadlineCommand {
        operation_id: DeadlineOperationId::new(),
        deadline_id: base.id,
        change: DeadlineChange::Correct {
            expected_revision: base.revision,
            definition,
            reason: FactText::new("Correct title").unwrap(),
        },
    }
}
pub fn retire(base: &DeadlineDetail) -> DeadlineCommand {
    DeadlineCommand {
        operation_id: DeadlineOperationId::new(),
        deadline_id: base.id,
        change: DeadlineChange::Retire {
            expected_revision: base.revision,
            reason: FactText::new("Withdraw registry").unwrap(),
        },
    }
}

/// Forge a self-consistent receipt to exercise chain checks independently of hashes.
pub fn change_action(db: &mut Fixture, detail: &DeadlineDetail, action: DeadlineAction) {
    let mut forged = detail.clone();
    forged.receipt.action = action;
    forged.status = if action == DeadlineAction::Retire {
        DeadlineStatus::Retired
    } else {
        DeadlineStatus::Active
    };
    let expected_revision = DeadlineRevision::new(forged.receipt.expected_revision).unwrap();
    let reason = forged.reason.clone().unwrap();
    let command = DeadlineCommand {
        operation_id: forged.receipt.operation_id,
        deadline_id: forged.id,
        change: if action == DeadlineAction::Retire {
            DeadlineChange::Retire {
                expected_revision,
                reason,
            }
        } else {
            DeadlineChange::SetAttention {
                expected_revision,
                attention: forged.attention.clone(),
                reason,
            }
        },
    };
    let review = deadline_review_bytes(&RingSha256Hasher, &forged).unwrap();
    let capture = deadline_capture_bytes(&RingSha256Hasher, &forged).unwrap();
    forged.receipt.review_digest = RingSha256Hasher.hash_bytes(&review);
    forged.receipt.capture_digest = RingSha256Hasher.hash_bytes(&capture);
    let submission = deadline_submission_bytes(
        forged.recorded_by.user_id().expect("human V1 fixture"),
        forged.case_id,
        &command,
        forged.receipt.review_digest,
    );
    forged.receipt.submission_digest = RingSha256Hasher.hash_bytes(&submission);
    deadline_receipt_matches(&RingSha256Hasher, &forged).unwrap();
    db.admin
        .execute(
            "UPDATE case_deadline_revisions SET action=$3,review_canonical=$4,
        review_digest=$5,capture_canonical=$6,capture_digest=$7,submission_canonical=$8,
        submission_digest=$9 WHERE deadline_id=$1 AND revision=$2",
            &[
                &detail.id.as_uuid(),
                &i64::from(detail.revision.get()),
                &action.as_str(),
                &review,
                &forged.receipt.review_digest.as_bytes().as_slice(),
                &capture,
                &forged.receipt.capture_digest.as_bytes().as_slice(),
                &submission,
                &forged.receipt.submission_digest.as_bytes().as_slice(),
            ],
        )
        .unwrap();
}
