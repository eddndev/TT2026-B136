use super::*;
use crate::{
    identity::Principal, typed_participants::ParticipantRevisionSnapshot, ApplicationError,
};
use domain::{
    clock::OffsetDateTime,
    crypto::{DocumentHasher, Sha256Digest},
    identity::Role,
    resource_hearings::ResourceHearingRevision,
};

pub(super) fn inconsistent(message: &str) -> ApplicationError {
    crate::resource_activities::ResourceActivityError::StoredInconsistent(message.into()).into()
}
pub(super) fn origin(h: &ResourceHearingDetail) -> ResourceHearingOrigin {
    let d = &h.review;
    ResourceHearingOrigin {
        case_id: d.case_id,
        resource_id: d.command.resource.id,
        hearing_id: d.command.hearing_id,
        operation_id: d.command.operation_id,
        association_id: d.command.association_id,
        submission_digest: d.submission_digest,
        capture_digest: h.capture_digest,
    }
}
/// Validates historical material without consulting current heads or requiring
/// the association to remain linked. Authorization belongs to the store call.
pub fn resource_hearing_creation_matches(
    hasher: &dyn DocumentHasher,
    result: &ResourceHearingCreation,
) -> Result<(), ApplicationError> {
    verify(hasher, result)
        .map_err(|_| inconsistent("resource hearing creation or origin differs from its evidence"))
}
fn verify(
    hasher: &dyn DocumentHasher,
    result: &ResourceHearingCreation,
) -> Result<(), ApplicationError> {
    let h = &result.hearing;
    let d = &h.review;
    if h.revision != ResourceHearingRevision::initial()
        || result.origin != origin(h)
        || d.recorded_by.email.is_empty()
        || d.recorded_by.email.trim() != d.recorded_by.email
        || d.recorded_by.email.chars().any(char::is_control)
    {
        return Err(inconsistent(
            "resource hearing origin, revision or author is invalid",
        ));
    }
    let actor = Principal {
        id: d.recorded_by.id,
        email: d.recorded_by.email.clone(),
        role: Role::Owner,
    };
    let reconstructed = super::preparation::prepare(
        hasher,
        &actor,
        d.case_id,
        d.command.resource.id,
        d.command.clone(),
        h.material.clone(),
    )?;
    if reconstructed != *d || capture_digest(hasher, h) != h.capture_digest {
        return Err(inconsistent("resource hearing capture differs from review"));
    }
    if h.recorded_at.offset() != time::UtcOffset::UTC || !(1..=9999).contains(&h.recorded_at.year())
    {
        return Err(inconsistent(
            "resource hearing clock must be representable UTC",
        ));
    }
    let m = &h.material;
    let mut latest = m.resource_head.recorded_at.max(m.resource.recorded_at);
    if let Some(act) = &m.act {
        latest = latest.max(act.recorded_at);
    }
    if let Some(admin) = m.administration.snapshot() {
        latest = latest.max(admin.changed_at);
    }
    for p in &m.participants {
        latest = latest.max(match &p.revision {
            ParticipantRevisionSnapshot::Manual(s) => s.changed_at,
            ParticipantRevisionSnapshot::Typed(s) => s.changed_at,
        });
        if let Some(subject) = &p.bound_subject {
            latest = latest.max(subject.changed_at);
        }
    }
    if h.recorded_at < latest {
        return Err(inconsistent(
            "resource hearing clock predates captured evidence",
        ));
    }
    Ok(())
}
pub(super) fn capture_digest(
    hasher: &dyn DocumentHasher,
    h: &ResourceHearingDetail,
) -> Sha256Digest {
    let mut bytes = b"RHCR1".to_vec();
    bytes.extend_from_slice(h.review.submission_digest.as_bytes());
    bytes.extend_from_slice(&h.revision.get().to_be_bytes());
    timestamp(&mut bytes, h.recorded_at);
    // Source values are committed in RHPR1; provenance not projected in that
    // review is additionally bound here, without asserting credential validity.
    bytes.extend_from_slice(&(h.material.participants.len() as u32).to_be_bytes());
    let mut participants = h.material.participants.iter().collect::<Vec<_>>();
    participants.sort_by_key(|p| (p.id().as_uuid(), p.revision_number().get()));
    for p in participants {
        bytes.extend_from_slice(p.id().as_uuid().as_bytes());
        bytes.extend_from_slice(&p.revision_number().get().to_be_bytes());
        match &p.revision {
            ParticipantRevisionSnapshot::Manual(s) => {
                bytes.push(0);
                timestamp(&mut bytes, s.changed_at);
                actor(&mut bytes, s.changed_by.id, &s.changed_by.email);
            }
            ParticipantRevisionSnapshot::Typed(s) => {
                bytes.push(1);
                timestamp(&mut bytes, s.changed_at);
                actor(&mut bytes, s.changed_by.id, &s.changed_by.email);
                bytes.extend_from_slice(s.submission_digest.as_bytes());
                bytes.extend_from_slice(&s.submission_revision.get().to_be_bytes());
                bytes.push(u8::from(s.credential_origin.is_some()));
                if let Some(c) = &s.credential_origin {
                    bytes.extend_from_slice(c.participant_id.as_uuid().as_bytes());
                    bytes.extend_from_slice(&c.participant_revision.get().to_be_bytes());
                    bytes.extend_from_slice(c.statement_digest.as_bytes());
                }
            }
        }
        bytes.push(u8::from(p.bound_subject.is_some()));
        if let Some(s) = &p.bound_subject {
            timestamp(&mut bytes, s.changed_at);
            actor(&mut bytes, s.changed_by.id, &s.changed_by.email);
        }
    }
    hasher.hash_bytes(&bytes)
}
fn timestamp(bytes: &mut Vec<u8>, at: OffsetDateTime) {
    bytes.extend_from_slice(&at.unix_timestamp().to_be_bytes());
    bytes.extend_from_slice(&at.nanosecond().to_be_bytes());
    bytes.extend_from_slice(&at.offset().whole_seconds().to_be_bytes());
}
fn actor(bytes: &mut Vec<u8>, id: domain::identity::UserId, email: &str) {
    bytes.extend_from_slice(id.as_uuid().as_bytes());
    bytes.extend_from_slice(&(email.len() as u64).to_be_bytes());
    bytes.extend_from_slice(email.as_bytes());
}
