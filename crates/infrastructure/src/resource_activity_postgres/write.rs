use super::{decode, port};
use application::{cases::CurrentCaseAdministration, resource_activities::*, ApplicationError};
use domain::crypto::DocumentHasher;
use postgres::Transaction;

pub(super) fn resource(detail: &ResourceActivityDetail) -> String {
    format!(
        "case:{}:resource:{}:association:{}:revision:{}:operation:{}:sha256:{}",
        detail.case_id,
        detail.resource_id,
        detail.id,
        detail.revision.get(),
        detail.receipt.operation_id,
        detail.receipt.capture_digest.to_hex()
    )
}
pub(super) fn insert(
    tx: &mut Transaction<'_>,
    d: &ResourceActivityDetail,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    if d.receipt.action == ResourceActivityAction::Link {
        tx.execute("INSERT INTO case_resource_activity_associations(id,case_id,resource_id) VALUES($1,$2,$3)",
            &[&d.id.as_uuid(),&d.case_id.as_uuid(),&d.resource_id.as_uuid()]).map_err(port)?;
    }
    let selected = d.selection;
    let (hearing, hearing_revision, hearing_digest, deadline, deadline_revision, deadline_digest) =
        match selected.target {
            ResourceActivityTarget::Hearing {
                id,
                revision,
                submission_digest,
            } => (
                Some(id.as_uuid()),
                Some(i64::from(revision.get())),
                Some(submission_digest.as_bytes().to_vec()),
                None,
                None,
                None,
            ),
            ResourceActivityTarget::Deadline {
                id,
                revision,
                capture_digest,
            } => (
                None,
                None,
                None,
                Some(id.as_uuid()),
                Some(i64::from(revision.get())),
                Some(capture_digest.as_bytes().to_vec()),
            ),
        };
    let (admin_revision, admin_digest, title, reference) = match &d.recorded_administration {
        CurrentCaseAdministration::Unrevised(value) => {
            (None, None, Some(value.title()), Some(value.reference()))
        }
        CurrentCaseAdministration::Recorded(value) => (
            Some(i64::from(value.revision.get())),
            Some(value.values_digest.as_bytes().as_slice()),
            None,
            None,
        ),
    };
    tx.execute("INSERT INTO case_resource_activity_association_revisions(
        association_id,case_id,resource_id,revision,operation_id,action,status,reason,resource_revision,resource_capture_digest,
        act_id,act_revision,act_resource_revision,act_capture_digest,target_kind,hearing_id,hearing_revision,hearing_submission_digest,
        deadline_id,deadline_revision,deadline_capture_digest,selection_canonical,submission_canonical,submission_digest,capture_canonical,capture_digest,
        previous_capture_digest,recorded_resource_revision,recorded_resource_capture_digest,recorded_administration_revision,
        recorded_administration_digest,recorded_administration_title,recorded_administration_reference,recorded_at_seconds,recorded_at_nanoseconds,recorded_by,recorded_by_email)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27,$28,$29,$30,$31,$32,$33,$34,$35,$36,$37)",
        &[&d.id.as_uuid(),&d.case_id.as_uuid(),&d.resource_id.as_uuid(),&i64::from(d.revision.get()),&d.receipt.operation_id.as_uuid(),&d.receipt.action.as_str(),&d.status.as_str(),&d.reason.as_ref().map(|value|value.as_str()),
        &i64::from(selected.resource.revision.get()),&selected.resource.capture_digest.as_bytes().as_slice(),
        &selected.act.map(|act|act.id.as_uuid()),&selected.act.map(|act|i64::from(act.revision.get())),&selected.act.map(|act|i64::from(act.resource_revision.get())),&selected.act.map(|act|act.capture_digest.as_bytes().to_vec()),
        &selected.target.kind().as_str(),&hearing,&hearing_revision,&hearing_digest,&deadline,&deadline_revision,&deadline_digest,
        &selected.canonical_bytes(),&resource_activity_submission_bytes(hasher,&decode::draft(d)?)?,&d.receipt.submission_digest.as_bytes().as_slice(),&resource_activity_capture_bytes(d),&d.receipt.capture_digest.as_bytes().as_slice(),
        &d.receipt.previous.map(|value|value.capture_digest.as_bytes().to_vec()),&i64::from(d.recorded_resource_head.revision.get()),&d.recorded_resource_head.capture_digest.as_bytes().as_slice(),
        &admin_revision,&admin_digest,&title,&reference,&d.recorded_at.unix_timestamp(),&(d.recorded_at.nanosecond() as i32),&d.recorded_by.id.as_uuid(),&d.recorded_by.email]).map_err(port)?;
    Ok(())
}
