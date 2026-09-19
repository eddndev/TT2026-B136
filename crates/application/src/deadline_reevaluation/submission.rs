use super::{validation, wire, wire::Reader, *};
use domain::{
    deadlines::{DeadlineId, DeadlineOperationId},
    identity::UserId,
};

type Result<T> = std::result::Result<T, TrackingCodecError>;

/// Encode only DLTX2. UUIDs and UTF-8 text retain their exact byte values.
/// The predecessor binds R-1; the capture for the new revision is a separate
/// commitment because authorized administrative data may advance on commit.
pub fn encode_tracked_submission(value: &TrackedSubmission) -> Result<Vec<u8>> {
    validation::submission(value)?;
    let mut bytes = b"DLTX2".to_vec();
    wire::case_id(&mut bytes, value.case_id);
    wire::uuid(&mut bytes, value.deadline_id.as_uuid());
    wire::uuid(&mut bytes, value.operation_id.as_uuid());
    bytes.push(value.action as u8);
    bytes.extend_from_slice(&value.expected_revision.to_be_bytes());
    bytes.extend_from_slice(value.review_digest.as_bytes());
    bytes.extend_from_slice(value.observations_digest.as_bytes());
    wire::optional(&mut bytes, value.predecessor, |bytes, previous| {
        bytes.extend_from_slice(previous.submission_digest.as_bytes());
        bytes.extend_from_slice(previous.capture_digest.as_bytes());
    });
    match &value.author {
        TrackedAuthor::User { id, email } => {
            bytes.push(0);
            wire::uuid(&mut bytes, id.as_uuid());
            wire::text(&mut bytes, email);
        }
        TrackedAuthor::Technical {
            service,
            policy_version,
        } => {
            bytes.extend([1, *service as u8]);
            bytes.extend_from_slice(&policy_version.to_be_bytes());
        }
    }
    wire::optional(&mut bytes, value.reason.as_deref(), wire::text);
    match value.cause {
        None => bytes.push(0),
        Some(TechnicalCause::SourceEvent { job_id, event }) => {
            bytes.push(1);
            wire::uuid(&mut bytes, job_id);
            bytes.extend_from_slice(&event.sequence.to_be_bytes());
            bytes.push(event.family as u8);
            wire::uuid(&mut bytes, event.source_id);
            bytes.extend_from_slice(&event.revision.to_be_bytes());
            wire::optional(&mut bytes, event.case_id, wire::case_id);
            wire::optional(&mut bytes, event.hearing_id, wire::uuid);
            wire::uuid(&mut bytes, event.operation_id);
        }
        Some(TechnicalCause::LegacyBootstrap {
            job_id,
            policy_version,
        }) => {
            bytes.push(2);
            wire::uuid(&mut bytes, job_id);
            bytes.extend_from_slice(&policy_version.to_be_bytes());
        }
    }
    if bytes.len() > MAX_TRACKED_SUBMISSION_BYTES {
        return Err(TrackingCodecError::SizeLimit);
    }
    Ok(bytes)
}

/// Decode DLTX2 strictly; malformed or unsupported versions never fall back to V1.
pub fn decode_tracked_submission(bytes: &[u8]) -> Result<TrackedSubmission> {
    let mut reader = Reader::new(bytes, b"DLTX2", MAX_TRACKED_SUBMISSION_BYTES)?;
    let case_id = reader.case_id()?;
    let deadline_id = DeadlineId::from_uuid(reader.uuid()?);
    let operation_id = DeadlineOperationId::from_uuid(reader.uuid()?);
    let action = match reader.byte()? {
        0 => TrackedAction::Register,
        1 => TrackedAction::Correct,
        2 => TrackedAction::SetAttention,
        3 => TrackedAction::Retire,
        4 => TrackedAction::Reevaluate,
        _ => return Err(TrackingCodecError::InvalidEncoding("action")),
    };
    let expected_revision = reader.u32()?;
    let review_digest = reader.digest()?;
    let observations_digest = reader.digest()?;
    let predecessor = reader.optional(|reader| {
        Ok(PredecessorReceipt {
            submission_digest: reader.digest()?,
            capture_digest: reader.digest()?,
        })
    })?;
    let author = match reader.byte()? {
        0 => TrackedAuthor::User {
            id: UserId::from_uuid(reader.uuid()?),
            email: reader.text(320)?,
        },
        1 => {
            let service = match reader.byte()? {
                0 => TechnicalService::DeadlineReevaluator,
                _ => return Err(TrackingCodecError::InvalidEncoding("technical service")),
            };
            TrackedAuthor::Technical {
                service,
                policy_version: reader.u16()?,
            }
        }
        _ => return Err(TrackingCodecError::InvalidEncoding("author")),
    };
    let reason = reader.optional(|reader| reader.text(1000))?;
    let cause = match reader.byte()? {
        0 => None,
        1 => Some(TechnicalCause::SourceEvent {
            job_id: reader.uuid()?,
            event: SourceEventReference {
                sequence: reader.u64()?,
                family: reader.family()?,
                source_id: reader.uuid()?,
                revision: reader.u32()?,
                case_id: reader.optional(Reader::case_id)?,
                hearing_id: reader.optional(Reader::uuid)?,
                operation_id: reader.uuid()?,
            },
        }),
        2 => Some(TechnicalCause::LegacyBootstrap {
            job_id: reader.uuid()?,
            policy_version: reader.u16()?,
        }),
        _ => return Err(TrackingCodecError::InvalidEncoding("cause")),
    };
    reader.finish()?;
    let value = TrackedSubmission {
        case_id,
        deadline_id,
        operation_id,
        action,
        expected_revision,
        review_digest,
        observations_digest,
        predecessor,
        author,
        reason,
        cause,
    };
    validation::submission(&value)?;
    Ok(value)
}
