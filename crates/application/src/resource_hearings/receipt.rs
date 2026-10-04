use super::*;
use crate::{
    identity::Principal,
    resource_activities::{
        resource_activity_command_from_detail, resource_activity_receipt_matches,
        ResourceActivityTargetDetail,
    },
    typed_participants::ParticipantRevisionSnapshot,
    ApplicationError,
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
/// Validates the original creation without consulting the current association
/// head. Later organizational unlinking does not replace its initial receipt.
pub fn resource_hearing_creation_matches(
    hasher: &dyn DocumentHasher,
    result: &ResourceHearingCreation,
) -> Result<(), ApplicationError> {
    verify_creation(hasher, result)
        .map_err(|_| inconsistent("resource hearing creation or origin differs from its evidence"))
}
fn verify_creation(
    hasher: &dyn DocumentHasher,
    result: &ResourceHearingCreation,
) -> Result<(), ApplicationError> {
    let h = &result.hearing;
    resource_hearing_receipt_matches(hasher, h)?;
    let a = &result.association;
    resource_activity_receipt_matches(hasher, a)?;
    if result.origin != origin(h)
        || a.case_id != h.review.case_id
        || a.resource_id != h.review.command.resource.id
        || resource_activity_command_from_detail(a)? != super::creation::association_command(h)
        || a.sources.resource != h.review.resource
        || a.sources.act != h.review.act
        || !matches!(&a.sources.target,
            ResourceActivityTargetDetail::ResourceHearing(captured) if same_capture(captured, h))
        || a.recorded_at != h.recorded_at
        || a.recorded_by != h.review.recorded_by
        || a.recorded_administration != h.review.observed_administration
        || a.recorded_resource_head != h.review.observed_resource_head
    {
        return Err(inconsistent(
            "initial association differs from resource hearing creation",
        ));
    }
    Ok(())
}

fn same_capture(left: &ResourceHearingDetail, right: &ResourceHearingDetail) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    // Material order is incidental; each person's exact provenance remains
    // attached to its identity and is independently committed by RHCR1.
    for hearing in [&mut left, &mut right] {
        hearing
            .material
            .participants
            .sort_by_key(|person| (person.id().as_uuid(), person.revision_number().get()));
    }
    left == right
}

/// Validates a historical hearing without an association or origin lookup.
/// This is the finite source boundary used by association receipt validation.
pub fn resource_hearing_receipt_matches(
    hasher: &dyn DocumentHasher,
    hearing: &ResourceHearingDetail,
) -> Result<(), ApplicationError> {
    verify_detail(hasher, hearing)
        .map_err(|_| inconsistent("resource hearing capture differs from its evidence"))
}
fn verify_detail(
    hasher: &dyn DocumentHasher,
    h: &ResourceHearingDetail,
) -> Result<(), ApplicationError> {
    let d = &h.review;
    if h.revision != ResourceHearingRevision::initial()
        || d.recorded_by.email.is_empty()
        || d.recorded_by.email.trim() != d.recorded_by.email
        || d.recorded_by.email.chars().any(char::is_control)
    {
        return Err(inconsistent(
            "resource hearing revision or author is invalid",
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
    hasher.hash_bytes(&resource_hearing_capture_bytes(h))
}

/// Stable capture framing commits the review, timestamp and source provenance.
pub fn resource_hearing_capture_bytes(h: &ResourceHearingDetail) -> Vec<u8> {
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
    bytes
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
