use super::ResourceHearingDraft;
use crate::{cases::case_administration_digest, procedural_facts::*, ApplicationError};
use domain::crypto::{DocumentHasher, Sha256Digest};

pub(super) fn digest(
    hasher: &dyn DocumentHasher,
    draft: &ResourceHearingDraft,
) -> Result<Sha256Digest, ApplicationError> {
    Ok(hasher.hash_bytes(&resource_hearing_submission_bytes(hasher, draft)?))
}

/// Stable review framing independent of JSON storage and commit time.
pub fn resource_hearing_submission_bytes(
    hasher: &dyn DocumentHasher,
    draft: &ResourceHearingDraft,
) -> Result<Vec<u8>, ApplicationError> {
    let mut bytes = b"RHPR1".to_vec();
    let command = &draft.command;
    for id in [
        draft.case_id.as_uuid(),
        command.operation_id.as_uuid(),
        command.hearing_id.as_uuid(),
        command.association_id.as_uuid(),
        draft.recorded_by.id.as_uuid(),
    ] {
        bytes.extend_from_slice(id.as_bytes());
    }
    blob(&mut bytes, draft.recorded_by.email.as_bytes());
    bytes.extend_from_slice(&command.expected_resource_revision.get().to_be_bytes());
    for reference in [command.resource, draft.observed_resource_head] {
        bytes.extend_from_slice(reference.id.as_uuid().as_bytes());
        bytes.extend_from_slice(&reference.revision.get().to_be_bytes());
        bytes.extend_from_slice(reference.capture_digest.as_bytes());
    }
    bytes.push(u8::from(command.act.is_some()));
    if let Some(act) = command.act {
        bytes.extend_from_slice(act.id.as_uuid().as_bytes());
        bytes.extend_from_slice(&act.revision.get().to_be_bytes());
        bytes.extend_from_slice(&act.resource_revision.get().to_be_bytes());
        bytes.extend_from_slice(act.capture_digest.as_bytes());
    }
    blob(&mut bytes, &command.values.canonical_bytes());
    let administration = &draft.observed_administration;
    bytes
        .extend_from_slice(case_administration_digest(hasher, &administration.values()).as_bytes());
    bytes.push(u8::from(administration.snapshot().is_some()));
    if let Some(snapshot) = administration.snapshot() {
        bytes.extend_from_slice(snapshot.case_id.as_uuid().as_bytes());
        bytes.extend_from_slice(&snapshot.revision.get().to_be_bytes());
        bytes.extend_from_slice(snapshot.values_digest.as_bytes());
        bytes.extend_from_slice(snapshot.changed_by.id.as_uuid().as_bytes());
        blob(&mut bytes, snapshot.changed_by.email.as_bytes());
        bytes.extend_from_slice(&snapshot.changed_at.unix_timestamp().to_be_bytes());
        bytes.extend_from_slice(&snapshot.changed_at.nanosecond().to_be_bytes());
        bytes.extend_from_slice(&snapshot.changed_at.offset().whole_seconds().to_be_bytes());
    }
    let mut sources = FactSources {
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
        direct_supports: vec![draft.support.clone()],
    };
    blob(&mut bytes, &fact_sources_bytes(&sources)?);
    sources.direct_supports.clear();
    bytes.extend_from_slice(&(draft.participants.len() as u32).to_be_bytes());
    for person in &draft.participants {
        sources.resolved.participants = vec![person.snapshot];
        sources.views.participants = vec![person.overview.clone()];
        blob(&mut bytes, &fact_sources_bytes(&sources)?);
    }
    Ok(bytes)
}
fn blob(bytes: &mut Vec<u8>, value: &[u8]) {
    bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
    bytes.extend_from_slice(value);
}
