use super::port;
use crate::procedural_resource_codec as codec;
use application::{cases::CurrentCaseAdministration, procedural_resources::*, ApplicationError};
use domain::crypto::DocumentHasher;
use postgres::Transaction;

pub(super) fn action(value: ResourceAction) -> &'static str {
    match value {
        ResourceAction::Register => "register",
        ResourceAction::Correct => "correct",
        ResourceAction::RecordAct => "record_act",
        ResourceAction::CorrectAct => "correct_act",
        ResourceAction::Archive => "archive",
        ResourceAction::Reactivate => "reactivate",
    }
}
pub(super) fn resource(detail: &ResourceDetail) -> String {
    format!(
        "case:{}:resource:{}:revision:{}:operation:{}:sha256:{}",
        detail.case_id,
        detail.id,
        detail.revision.get(),
        detail.receipt.operation_id,
        detail.receipt.capture_digest.to_hex()
    )
}
pub(super) fn insert(
    tx: &mut Transaction<'_>,
    detail: &ResourceDetail,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let d = detail;
    if d.receipt.action == ResourceAction::Register {
        tx.execute(
            "INSERT INTO case_procedural_resources(id,case_id) VALUES($1,$2)",
            &[&d.id.as_uuid(), &d.case_id.as_uuid()],
        )
        .map_err(port)?;
    }
    if d.receipt.action == ResourceAction::RecordAct {
        if let Some(act) = &d.act {
            tx.execute("INSERT INTO case_procedural_resource_acts(id,resource_id,case_id,initial_resource_revision) VALUES($1,$2,$3,$4)",
                &[&act.id.as_uuid(),&d.id.as_uuid(),&d.case_id.as_uuid(),&i64::from(d.revision.get())]).map_err(port)?;
        }
    }
    let (admin_revision, admin_digest, title, reference) = match &d.recorded_administration {
        CurrentCaseAdministration::Unrevised(v) => {
            (None, None, Some(v.title()), Some(v.reference()))
        }
        CurrentCaseAdministration::Recorded(v) => (
            Some(i64::from(v.revision.get())),
            Some(v.values_digest.as_bytes().as_slice()),
            None,
            None,
        ),
    };
    let draft = ResourceDraft {
        case_id: d.case_id,
        command: resource_command_from_detail(d)?,
        result_revision: d.revision,
        values: d.values.clone(),
        status: d.status,
        sources: d.sources.clone(),
        act: d.act.clone(),
        previous: d.receipt.previous,
        recorded_by: d.recorded_by.clone(),
        observed_administration: d.recorded_administration.clone(),
        observed_stage: d.recorded_stage.clone(),
        submission_digest: d.receipt.submission_digest,
    };
    tx.execute("INSERT INTO case_procedural_resource_revisions(resource_id,case_id,revision,operation_id,action,status,kind,reason,
        values_canonical,values_view,values_digest,sources_canonical,sources_digest,supports_view,submission_canonical,submission_digest,
        capture_canonical,capture_digest,previous_capture_digest,act_id,act_revision,act_values_canonical,act_values_view,act_supports_view,
        act_previous_resource_revision,act_previous_capture_digest,recorded_administration_revision,recorded_administration_digest,
        recorded_administration_title,recorded_administration_reference,recorded_stage_revision,recorded_at_seconds,recorded_at_nanoseconds,recorded_by,recorded_by_email)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27,$28,$29,$30,$31,$32,$33,$34,$35)",
        &[&d.id.as_uuid(),&d.case_id.as_uuid(),&i64::from(d.revision.get()),&d.receipt.operation_id.as_uuid(),&action(d.receipt.action),&d.status.as_str(),&d.values.kind().as_str(),&d.reason.as_ref().map(|r|r.as_str()),
        &d.values.canonical_bytes(),&codec::encode_values(&d.values),&d.receipt.values_digest.as_bytes().as_slice(),&resource_sources_bytes(&d.sources)?,&d.receipt.sources_digest.as_bytes().as_slice(),&codec::encode_supports(&d.sources.supports),
        &resource_submission_bytes(hasher,&draft)?,&d.receipt.submission_digest.as_bytes().as_slice(),&resource_capture_bytes(d),&d.receipt.capture_digest.as_bytes().as_slice(),&d.receipt.previous.map(|p|p.capture_digest.as_bytes().to_vec()),
        &d.act.as_ref().map(|a|a.id.as_uuid()),&d.act.as_ref().map(|a|i64::from(a.revision.get())),&d.act.as_ref().map(|a|a.values.canonical_bytes()),&d.act.as_ref().map(|a|codec::encode_act(&a.values)),&d.act.as_ref().map(|a|codec::encode_supports(&a.supports)),
        &d.act.as_ref().and_then(|a|a.previous.map(|p|i64::from(p.revision.get()))),&d.act.as_ref().and_then(|a|a.previous.map(|p|p.capture_digest.as_bytes().to_vec())),
        &admin_revision,&admin_digest,&title,&reference,&d.recorded_stage.revision().map(|r|i64::from(r.get())),&d.recorded_at.unix_timestamp(),&(d.recorded_at.nanosecond() as i32),&d.recorded_by.id.as_uuid(),&d.recorded_by.email]).map_err(port)?;
    Ok(())
}
