use super::port;
use application::{cases::CurrentCaseAdministration, resource_hearings::*, ApplicationError};
use domain::crypto::DocumentHasher;
use postgres::Transaction;
pub(super) fn insert(
    tx: &mut Transaction<'_>,
    h: &ResourceHearingDetail,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let d = &h.review;
    let c = &d.command;
    tx.execute(
        "INSERT INTO case_resource_hearings(id,case_id,resource_id) VALUES($1,$2,$3)",
        &[
            &c.hearing_id.as_uuid(),
            &d.case_id.as_uuid(),
            &c.resource.id.as_uuid(),
        ],
    )
    .map_err(port)?;
    let (admin_revision, admin_digest, title, reference) = match &d.observed_administration {
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
    tx.execute("INSERT INTO case_resource_hearing_revisions(hearing_id,case_id,resource_id,revision,operation_id,association_id,
        resource_revision,resource_capture_digest,act_id,act_revision,act_resource_revision,act_capture_digest,
        recorded_resource_revision,recorded_resource_capture_digest,values_view,values_canonical,
        submission_canonical,submission_digest,capture_canonical,capture_digest,
        recorded_administration_revision,recorded_administration_digest,recorded_administration_title,recorded_administration_reference,
        recorded_at_seconds,recorded_at_nanoseconds,recorded_by,recorded_by_email)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27,$28)",
        &[&c.hearing_id.as_uuid(),&d.case_id.as_uuid(),&c.resource.id.as_uuid(),&i64::from(h.revision.get()),&c.operation_id.as_uuid(),&c.association_id.as_uuid(),
        &i64::from(c.resource.revision.get()),&c.resource.capture_digest.as_bytes().as_slice(),&c.act.map(|a|a.id.as_uuid()),&c.act.map(|a|i64::from(a.revision.get())),
        &c.act.map(|a|i64::from(a.resource_revision.get())),&c.act.map(|a|a.capture_digest.as_bytes().to_vec()),
        &i64::from(d.observed_resource_head.revision.get()),&d.observed_resource_head.capture_digest.as_bytes().as_slice(),
        &crate::resource_hearing_codec::view(&c.values),&c.values.canonical_bytes(),&resource_hearing_submission_bytes(hasher,d)?,&d.submission_digest.as_bytes().as_slice(),
        &resource_hearing_capture_bytes(h),&h.capture_digest.as_bytes().as_slice(),&admin_revision,&admin_digest,&title,&reference,
        &h.recorded_at.unix_timestamp(),&(h.recorded_at.nanosecond() as i32),&d.recorded_by.id.as_uuid(),&d.recorded_by.email]).map_err(port)?;
    Ok(())
}
