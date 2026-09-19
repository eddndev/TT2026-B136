#![allow(dead_code)]
use crate::{deadline_backend_support::*, deadline_profile_database_support as profiles};
use application::{
    deadline_evaluations::{deadline_evaluation_input_bytes, deadline_evaluation_record_bytes},
    deadline_inputs::DeadlineInputHeads,
    deadline_profiles::{DeadlineProfileCollection, DeadlineProfileDetail},
    deadline_reevaluation::encode_observations,
    deadline_tracking::{TrackingPolicies, TrackingPolicy},
    deadlines::*,
};
use domain::{
    crypto::DocumentHasher,
    deadline_triggers::TriggerSourceRef,
    identity::{Role, UserId},
    procedural_facts::FactDeclaration,
    procedural_time::DeclaredProceduralTime,
};
use infrastructure::RingSha256Hasher;
use postgres::Error;
use serde_json::{json, Value};

pub fn fixed_profile() -> TrackingPolicies {
    TrackingPolicies {
        profile: TrackingPolicy::Fixed,
        source: TrackingPolicy::Follow,
        calendar: TrackingPolicy::Undetermined,
    }
}

pub fn advance_profile(db: &Fixture, base: &DeadlineProfileDetail) -> DeadlineProfileDetail {
    profiles::persist(
        &profiles::service(db, db.owner, Role::Owner),
        DeadlineProfileCollection::ForCase(db.case),
        profiles::replace(base),
    )
}

pub fn prepare_as(
    db: &Fixture,
    repository: &dyn DeadlineStore,
    command: &DeadlineCommand,
    actor: UserId,
    email: &str,
    policies: Option<TrackingPolicies>,
) -> PreparedDeadlineChange {
    let preparation = repository.prepare(actor, db.case, command).unwrap();
    prepare_tracked_deadline_change(
        &RingSha256Hasher,
        DeadlineActorSnapshot::User {
            id: actor,
            email: email.into(),
        },
        db.case,
        command.clone(),
        preparation,
        policies,
        None,
    )
    .unwrap()
}

/// Insert complete resolution-only test captures without re-preparing their observed heads.
/// Production commit tests separately prove that state and audit share a transaction.
pub fn insert(db: &Fixture, value: &DeadlineDetail) -> Result<(), Error> {
    deadline_receipt_matches(&RingSha256Hasher, value).unwrap();
    let tracking = value.tracking.as_ref().unwrap();
    let DeadlineActorSnapshot::User { id: actor, email } = &value.recorded_by else {
        panic!("human guard fixture requires a captured user")
    };
    let heads = DeadlineInputHeads::capture(&value.calculation.material);
    let (selected, head) = match (&value.definition.input.selection.source, heads.source) {
        (
            FactDeclaration::Known(TriggerSourceRef::Resolution(selected)),
            Some(TriggerSourceRef::Resolution(head)),
        ) => (*selected, head),
        _ => panic!("guard fixture requires a resolution source"),
    };
    assert!(value.definition.input.calendar.is_none());
    assert!(heads.calendar.is_none());
    let input = deadline_evaluation_input_bytes(&value.definition.input).unwrap();
    let result = deadline_evaluation_record_bytes(&value.calculation.result);
    let administration = &value.calculation.material.administration;
    let admin_revision = administration
        .revision()
        .map(|revision| i64::from(revision.get()));
    let admin_bytes = administration.values().canonical_bytes();
    let admin_digest = RingSha256Hasher.hash_bytes(&admin_bytes);
    let review = deadline_review_bytes(&RingSha256Hasher, value).unwrap();
    let capture = deadline_capture_bytes(&RingSha256Hasher, value).unwrap();
    let submission = deadline_record_submission_bytes(value).unwrap();
    let suffix_size = 102
        + 2 * tracking.review.reasons().len()
        + 4 * usize::from(tracking.administration.snapshot().is_some());
    let suffix = &capture[capture.len() - suffix_size..];
    let observations = encode_observations(&tracking.observations).unwrap();
    let due = value.calculation.result.due_at();
    let mut client = db.runtime();
    let mut tx = client.transaction()?;
    if value.revision.get() == 1 {
        tx.execute(
            "INSERT INTO case_deadlines(id,case_id) VALUES($1,$2)",
            &[&value.id.as_uuid(), &value.case_id.as_uuid()],
        )?;
    }
    tx.execute("INSERT INTO case_deadline_revisions(
        deadline_id,case_id,revision,title,profile_id,profile_revision,input_canonical,result_canonical,
        observed_administration_revision,observed_administration_canonical,observed_administration_digest,
        responsible_id,responsible_email,responsible_role,attention,operation_id,action,reason,
        review_canonical,review_digest,capture_canonical,capture_digest,submission_canonical,submission_digest,
        recorded_at_seconds,recorded_at_nanoseconds,recorded_by,recorded_by_email,
        source_kind,source_id,source_revision,source_head_revision,due_at_seconds,due_at_nanoseconds,
        tracking_canonical,observations_canonical)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,
            $21,$22,$23,$24,$25,$26,$27,$28,'resolution',$29,$30,$31,$32,$33,$34,$35)",
        &[&value.id.as_uuid(), &value.case_id.as_uuid(), &i64::from(value.revision.get()),
        &value.definition.title.as_str(), &value.definition.profile.id.as_uuid(),
        &i64::from(value.definition.profile.revision.get()), &input, &result, &admin_revision,
        &admin_bytes, &admin_digest.as_bytes().as_slice(), &value.responsible.id.as_uuid(),
        &value.responsible.email, &value.responsible.role.as_str(), &attention_json(&value.attention),
        &value.receipt.operation_id.as_uuid(), &value.receipt.action.as_str(),
        &value.reason.as_ref().map(|reason| reason.as_str()), &review,
        &value.receipt.review_digest.as_bytes().as_slice(), &capture,
        &value.receipt.capture_digest.as_bytes().as_slice(), &submission,
        &value.receipt.submission_digest.as_bytes().as_slice(), &value.recorded_at.unix_timestamp(),
        &(value.recorded_at.nanosecond() as i32), &actor.as_uuid(), &email,
        &selected.id.as_uuid(), &i64::from(selected.revision.get()), &i64::from(head.revision.get()),
        &due.map(|at| at.unix_timestamp()), &due.map(|at| at.nanosecond() as i32), &suffix, &observations])?;
    tx.commit()
}

fn attention_json(value: &DeadlineAttention) -> Value {
    match value {
        DeadlineAttention::Pending => json!({"status":"pending"}),
        DeadlineAttention::Recorded {
            occurred_at,
            statement,
            locator,
        } => {
            assert_eq!(*occurred_at, DeclaredProceduralTime::unknown());
            json!({"status":"recorded","occurred_at":{"precision":"unknown"},
                "statement":statement.as_str(),"locator":locator.as_str()})
        }
    }
}
