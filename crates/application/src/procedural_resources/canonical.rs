use super::*;
use crate::{
    case_stages::{CaseStageEntry, CurrentCaseStage, StageSupportSnapshot},
    cases::{case_administration_digest, CurrentCaseAdministration},
    procedural_facts::*,
    ApplicationError,
};
use domain::{clock::OffsetDateTime, crypto::DocumentHasher};

fn empty() -> FactSources {
    FactSources {
        resolved: FactResolvedSources {
            resolution: None,
            participants: vec![],
            hearing_results: vec![],
        },
        views: FactSourceViews {
            resolution: None,
            participants: vec![],
            hearing_results: vec![],
        },
        direct_supports: vec![],
    }
}
/// PRSS1 frames existing bounded PFSRC1 components without changing fact policies.
pub fn resource_sources_bytes(sources: &ResourceSources) -> Result<Vec<u8>, ApplicationError> {
    if sources.appellants.len() > 32 {
        return Err(inconsistent("resource has too many appellant captures"));
    }
    let mut bytes = b"PRSS1".to_vec();
    let mut parent = empty();
    parent.resolved.resolution = Some(sources.resolution.snapshot);
    parent.views.resolution = Some(sources.resolution.view.clone());
    blob(&mut bytes, &fact_sources_bytes(&parent)?);
    bytes.extend_from_slice(&(sources.appellants.len() as u32).to_be_bytes());
    let mut previous = None;
    for appellant in &sources.appellants {
        let key = (
            appellant.snapshot.reference.id.as_uuid(),
            appellant.snapshot.reference.revision.get(),
        );
        if previous.is_some_and(|old| old >= key) {
            return Err(inconsistent(
                "resource appellant captures are not strictly ordered",
            ));
        }
        previous = Some(key);
        let mut component = empty();
        component.resolved.participants.push(appellant.snapshot);
        component
            .views
            .participants
            .push(appellant.overview.clone());
        blob(&mut bytes, &fact_sources_bytes(&component)?);
    }
    blob(&mut bytes, &support_bytes(&sources.supports)?);
    Ok(bytes)
}
pub(super) fn support_bytes(
    supports: &[StageSupportSnapshot],
) -> Result<Vec<u8>, ApplicationError> {
    let mut component = empty();
    component.direct_supports = supports.to_vec();
    fact_sources_bytes(&component)
}
/// PRTX1 binds reviewed content, actor email, observations, previous receipt and act.
/// This is a receipt encoding; it is not a storage JSON format or legal assertion.
pub fn resource_submission_bytes(
    hasher: &dyn DocumentHasher,
    draft: &ResourceDraft,
) -> Result<Vec<u8>, ApplicationError> {
    let mut bytes = b"PRTX1".to_vec();
    bytes.extend_from_slice(draft.case_id.as_uuid().as_bytes());
    bytes.extend_from_slice(draft.command.resource_id.as_uuid().as_bytes());
    bytes.extend_from_slice(draft.command.operation_id.as_uuid().as_bytes());
    bytes.push(draft.command.action().tag());
    bytes.extend_from_slice(&draft.command.expected_revision().to_be_bytes());
    bytes.extend_from_slice(&draft.result_revision.get().to_be_bytes());
    bytes.extend_from_slice(draft.recorded_by.id.as_uuid().as_bytes());
    blob(&mut bytes, draft.recorded_by.email.as_bytes());
    bytes.push(draft.status.tag());
    blob(&mut bytes, &draft.values.canonical_bytes());
    blob(&mut bytes, &resource_sources_bytes(&draft.sources)?);
    previous(&mut bytes, draft.previous);
    bytes.push(u8::from(draft.command.reason().is_some()));
    if let Some(reason) = draft.command.reason() {
        blob(&mut bytes, reason.as_str().as_bytes());
    }
    bytes.push(u8::from(draft.act.is_some()));
    if let Some(act) = &draft.act {
        bytes.extend_from_slice(act.id.as_uuid().as_bytes());
        bytes.extend_from_slice(&act.revision.get().to_be_bytes());
        blob(&mut bytes, &act.values.canonical_bytes());
        blob(&mut bytes, &support_bytes(&act.supports)?);
        previous(&mut bytes, act.previous);
    }
    administration(&mut bytes, hasher, &draft.observed_administration);
    stage(&mut bytes, &draft.observed_stage)?;
    Ok(bytes)
}
/// PRCP1 binds the committed timestamp to the complete reviewed receipt.
pub fn resource_capture_bytes(detail: &ResourceDetail) -> Vec<u8> {
    let mut bytes = b"PRCP1".to_vec();
    bytes.extend_from_slice(detail.receipt.submission_digest.as_bytes());
    instant(&mut bytes, detail.recorded_at);
    bytes
}
pub(super) fn draft_from_detail(
    detail: &ResourceDetail,
    command: ResourceCommand,
) -> ResourceDraft {
    ResourceDraft {
        case_id: detail.case_id,
        command,
        result_revision: detail.revision,
        values: detail.values.clone(),
        status: detail.status,
        sources: detail.sources.clone(),
        act: detail.act.clone(),
        previous: detail.receipt.previous,
        recorded_by: detail.recorded_by.clone(),
        observed_administration: detail.recorded_administration.clone(),
        observed_stage: detail.recorded_stage.clone(),
        submission_digest: detail.receipt.submission_digest,
    }
}
fn previous(bytes: &mut Vec<u8>, value: Option<ResourceRevisionRef>) {
    bytes.push(u8::from(value.is_some()));
    if let Some(value) = value {
        bytes.extend_from_slice(&value.revision.get().to_be_bytes());
        bytes.extend_from_slice(value.capture_digest.as_bytes());
    }
}
fn administration(
    bytes: &mut Vec<u8>,
    hasher: &dyn DocumentHasher,
    admin: &CurrentCaseAdministration,
) {
    bytes.extend_from_slice(case_administration_digest(hasher, &admin.values()).as_bytes());
    bytes.push(u8::from(admin.snapshot().is_some()));
    if let Some(snapshot) = admin.snapshot() {
        bytes.extend_from_slice(snapshot.case_id.as_uuid().as_bytes());
        bytes.extend_from_slice(&snapshot.revision.get().to_be_bytes());
        bytes.extend_from_slice(snapshot.values_digest.as_bytes());
        bytes.extend_from_slice(snapshot.changed_by.id.as_uuid().as_bytes());
        blob(bytes, snapshot.changed_by.email.as_bytes());
        instant(bytes, snapshot.changed_at);
    }
}
fn stage(bytes: &mut Vec<u8>, stage: &CurrentCaseStage) -> Result<(), ApplicationError> {
    let Some(entry) = stage.entry() else {
        bytes.push(0);
        return Ok(());
    };
    match entry {
        CaseStageEntry::Initial(value) => {
            bytes.push(1);
            bytes.extend_from_slice(value.case_id.as_uuid().as_bytes());
            bytes.extend_from_slice(&value.stage_revision.get().to_be_bytes());
            bytes.extend_from_slice(&value.administration_revision.get().to_be_bytes());
            bytes.extend_from_slice(value.administration_digest.as_bytes());
        }
        CaseStageEntry::Changed(value) => {
            bytes.push(2);
            bytes.extend_from_slice(value.case_id.as_uuid().as_bytes());
            bytes.extend_from_slice(&value.stage_revision.get().to_be_bytes());
            blob(
                bytes,
                value
                    .from_stage
                    .map(|s| s.as_str())
                    .unwrap_or("")
                    .as_bytes(),
            );
            blob(bytes, &value.values.canonical_bytes());
            bytes.extend_from_slice(value.values_digest.as_bytes());
            bytes.extend_from_slice(&value.administration_revision.get().to_be_bytes());
            bytes.extend_from_slice(value.administration_digest.as_bytes());
            blob(bytes, &support_bytes(&value.supports)?);
        }
    }
    bytes.extend_from_slice(entry.recorded_by().id.as_uuid().as_bytes());
    blob(bytes, entry.recorded_by().email.as_bytes());
    instant(bytes, entry.recorded_at());
    Ok(())
}
fn instant(bytes: &mut Vec<u8>, at: OffsetDateTime) {
    bytes.extend_from_slice(&at.unix_timestamp_nanos().to_be_bytes());
    bytes.extend_from_slice(&at.offset().whole_seconds().to_be_bytes());
}
fn blob(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
    bytes.extend_from_slice(value);
}
