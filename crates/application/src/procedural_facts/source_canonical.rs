use super::{source_encoding::*, source_shape, *};
use crate::{
    case_stages::StageSupportSnapshot,
    documents::{StageDocumentFormat, StageFormatPolicy},
    typed_participants::ParticipantOverview,
    ApplicationError,
};
use domain::{
    crypto::{DocumentHasher, Sha256Digest},
    participants::DirectoryStatus,
};

/// Encodes bounded, ordered sources and matching readable views as PFSRC1.
/// This validates shape, not source existence, authorization or material derivation.
pub fn fact_sources_bytes(sources: &FactSources) -> Result<Vec<u8>, ApplicationError> {
    source_shape::validate(sources)?;
    let mut bytes = b"PFSRC1".to_vec();
    optional(
        &mut bytes,
        sources.resolved.resolution.as_ref(),
        |bytes, source| {
            if let Some(view) = &sources.views.resolution {
                resolution(bytes, source, view);
            }
        },
    );
    bytes.extend_from_slice(&(sources.resolved.participants.len() as u32).to_be_bytes());
    for (source, view) in sources
        .resolved
        .participants
        .iter()
        .zip(&sources.views.participants)
    {
        participant(&mut bytes, source, view);
    }
    bytes.extend_from_slice(&(sources.resolved.hearing_results.len() as u32).to_be_bytes());
    for (source, view) in sources
        .resolved
        .hearing_results
        .iter()
        .zip(&sources.views.hearing_results)
    {
        hearing(&mut bytes, source, view);
    }
    bytes.extend_from_slice(&(sources.direct_supports.len() as u32).to_be_bytes());
    for support in &sources.direct_supports {
        document(&mut bytes, support);
    }
    Ok(bytes)
}

pub fn fact_sources_digest(
    hasher: &dyn DocumentHasher,
    sources: &FactSources,
) -> Result<Sha256Digest, ApplicationError> {
    Ok(hasher.hash_bytes(&fact_sources_bytes(sources)?))
}

fn resolution(
    bytes: &mut Vec<u8>,
    source: &FactResolutionSourceSnapshot,
    view: &FactResolutionView,
) {
    bytes.extend_from_slice(source.case_id.as_uuid().as_bytes());
    bytes.extend_from_slice(source.reference.id.as_uuid().as_bytes());
    bytes.extend_from_slice(&source.reference.revision.get().to_be_bytes());
    bytes.extend_from_slice(source.values_digest.as_bytes());
    bytes.extend_from_slice(source.submission_digest.as_bytes());
    bytes.push(status(source.status));
    declaration(bytes, &view.class, |bytes, class| match class {
        ResolutionClass::Order => bytes.push(0),
        ResolutionClass::Judgment => bytes.push(1),
        ResolutionClass::Other(label) => {
            bytes.push(2);
            text(bytes, label.as_str());
        }
    });
    declaration(bytes, &view.issuer, |bytes, label| {
        text(bytes, label.as_str())
    });
    procedural_time(bytes, view.issued_at);
    text(bytes, view.summary.as_str());
}
fn participant(bytes: &mut Vec<u8>, source: &FactParticipantSnapshot, view: &ParticipantOverview) {
    bytes.extend_from_slice(source.case_id.as_uuid().as_bytes());
    bytes.extend_from_slice(source.reference.id.as_uuid().as_bytes());
    bytes.extend_from_slice(&source.reference.revision.get().to_be_bytes());
    bytes.extend_from_slice(source.values_digest.as_bytes());
    bytes.push(match source.status {
        DirectoryStatus::Active => 0,
        DirectoryStatus::Archived => 1,
    });
    optional(bytes, source.subject.as_ref(), |bytes, subject| {
        bytes.extend_from_slice(subject.id.as_uuid().as_bytes());
        bytes.extend_from_slice(&subject.revision.get().to_be_bytes());
        bytes.extend_from_slice(subject.values_digest.as_bytes());
    });
    text(bytes, &view.display_name);
    text(bytes, &view.procedural_role);
    optional(bytes, view.organization.as_ref(), |bytes, value| {
        text(bytes, value)
    });
    optional(bytes, view.kind.as_ref(), |bytes, value| {
        bytes.push(value.tag())
    });
}
fn hearing(bytes: &mut Vec<u8>, source: &FactHearingSourceSnapshot, view: &FactHearingView) {
    bytes.extend_from_slice(source.case_id.as_uuid().as_bytes());
    let reference = source.reference;
    bytes.extend_from_slice(reference.hearing_id.as_uuid().as_bytes());
    bytes.extend_from_slice(reference.result_id.as_uuid().as_bytes());
    bytes.extend_from_slice(&reference.revision.get().to_be_bytes());
    optional(bytes, reference.agreement_id.as_ref(), |bytes, id| {
        bytes.extend_from_slice(id.as_uuid().as_bytes())
    });
    bytes.extend_from_slice(source.values_digest.as_bytes());
    bytes.extend_from_slice(source.submission_digest.as_bytes());
    bytes.push(source.status.tag());
    bytes.push(view.occurrence.tag());
    hearing_time(bytes, view.event_time);
    text(bytes, view.summary.as_str());
    optional(bytes, view.agreement.as_ref(), |bytes, value| {
        text(bytes, value.text().as_str())
    });
}
fn document(bytes: &mut Vec<u8>, support: &StageSupportSnapshot) {
    bytes.extend_from_slice(support.reference.id.as_uuid().as_bytes());
    bytes.extend_from_slice(&support.reference.version.get().to_be_bytes());
    bytes.extend_from_slice(support.digest.as_bytes());
    text(bytes, &support.name);
    bytes.push(match support.format {
        StageDocumentFormat::Pdf => 0,
        StageDocumentFormat::Docx => 1,
    });
    bytes.push(match support.policy {
        StageFormatPolicy::PdfDocxV1 => 0,
    });
}
