use super::{evidence::text, *};
use crate::{
    deadline_evaluations::{deadline_evaluation_input_bytes, deadline_evaluation_record_bytes},
    ApplicationError,
};
use domain::{
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
    identity::UserId,
    procedural_facts::FactText,
};

#[derive(Clone, Copy)]
pub(super) struct Content<'a> {
    pub definition: &'a DeadlineDefinition,
    pub calculation: &'a DeadlineCalculation,
    pub responsible: &'a DeadlineResponsibleSnapshot,
    pub attention: &'a DeadlineAttention,
    pub status: DeadlineStatus,
    pub tracking: Option<&'a DeadlineTrackingCapture>,
}
impl<'a> From<&'a DeadlineDetail> for Content<'a> {
    fn from(value: &'a DeadlineDetail) -> Self {
        Self {
            definition: &value.definition,
            calculation: &value.calculation,
            responsible: &value.responsible,
            attention: &value.attention,
            status: value.status,
            tracking: value.tracking.as_ref(),
        }
    }
}
impl<'a> From<&'a PreparedDeadlineChange> for Content<'a> {
    fn from(value: &'a PreparedDeadlineChange) -> Self {
        Self {
            definition: &value.definition,
            calculation: &value.calculation,
            responsible: &value.responsible,
            attention: &value.attention,
            status: value.status(),
            tracking: value.tracking.as_ref(),
        }
    }
}
pub(super) fn state_digest(
    hasher: &dyn DocumentHasher,
    content: Content<'_>,
    include_observed_administration: bool,
) -> Result<Sha256Digest, ApplicationError> {
    Ok(hasher.hash_bytes(&state_bytes(
        hasher,
        content,
        include_observed_administration,
    )?))
}
/// Encode the complete reviewed state for storage alongside its digest.
/// This preserves captured evidence; it does not recalculate or validate the receipt.
pub fn deadline_review_bytes(
    hasher: &dyn DocumentHasher,
    detail: &DeadlineDetail,
) -> Result<Vec<u8>, ApplicationError> {
    state_bytes(hasher, Content::from(detail), false)
}
/// Encode the reviewed state and the exact observed administration for storage.
pub fn deadline_capture_bytes(
    hasher: &dyn DocumentHasher,
    detail: &DeadlineDetail,
) -> Result<Vec<u8>, ApplicationError> {
    state_bytes(hasher, Content::from(detail), true)
}
fn state_bytes(
    hasher: &dyn DocumentHasher,
    content: Content<'_>,
    include_observed_administration: bool,
) -> Result<Vec<u8>, ApplicationError> {
    let tracking_digest = content
        .tracking
        .map(|tracking| {
            super::tracked_state::validate(
                hasher,
                content.definition,
                content.calculation,
                tracking,
            )
        })
        .transpose()?;
    let mut bytes = match (content.tracking.is_some(), include_observed_administration) {
        (false, false) => b"DLRV1",
        (false, true) => b"DLST1",
        (true, false) => b"DLRV2",
        (true, true) => b"DLST2",
    }
    .to_vec();
    let definition = content.definition;
    text(&mut bytes, definition.title.as_str());
    bytes.extend_from_slice(definition.profile.id.as_uuid().as_bytes());
    bytes.extend_from_slice(&definition.profile.revision.get().to_be_bytes());
    bytes.extend_from_slice(definition.responsible.as_uuid().as_bytes());
    bytes.extend_from_slice(
        hasher
            .hash_bytes(&deadline_evaluation_input_bytes(&definition.input)?)
            .as_bytes(),
    );
    super::evidence::calculation(
        &mut bytes,
        hasher,
        content.calculation,
        include_observed_administration,
    );
    bytes.extend_from_slice(
        hasher
            .hash_bytes(&deadline_evaluation_record_bytes(
                &content.calculation.result,
            ))
            .as_bytes(),
    );
    bytes.extend_from_slice(content.responsible.id.as_uuid().as_bytes());
    text(&mut bytes, &content.responsible.email);
    text(&mut bytes, content.responsible.role.as_str());
    match content.attention {
        DeadlineAttention::Pending => bytes.push(0),
        DeadlineAttention::Recorded {
            occurred_at,
            statement,
            locator,
        } => {
            bytes.push(1);
            crate::deadline_inputs::encoding::write::declared_time(&mut bytes, *occurred_at);
            text(&mut bytes, statement.as_str());
            text(&mut bytes, locator.as_str());
        }
    }
    bytes.push(u8::from(content.status == DeadlineStatus::Retired));
    if let (Some(tracking), Some(digest)) = (content.tracking, tracking_digest) {
        super::tracked_state::append(
            &mut bytes,
            hasher,
            tracking,
            digest,
            include_observed_administration,
        );
    }
    Ok(bytes)
}
struct SubmissionHeader<'a> {
    actor: UserId,
    case_id: CaseId,
    id: DeadlineId,
    operation_id: DeadlineOperationId,
    action: DeadlineAction,
    expected_revision: u32,
    reason: Option<&'a FactText>,
}
/// DLTX1 binds the operation to the reviewed state. The capture digest additionally
/// binds the administrative observation preserved atomically by the store.
pub fn deadline_submission_bytes(
    actor: UserId,
    case_id: CaseId,
    command: &DeadlineCommand,
    review_digest: Sha256Digest,
) -> Vec<u8> {
    encode(
        SubmissionHeader {
            actor,
            case_id,
            id: command.deadline_id,
            operation_id: command.operation_id,
            action: command.action(),
            expected_revision: command.expected_revision(),
            reason: command.reason(),
        },
        review_digest,
    )
}
pub(super) fn submission_digest(
    hasher: &dyn DocumentHasher,
    actor: UserId,
    case_id: CaseId,
    id: DeadlineId,
    receipt: &DeadlineReceipt,
    state_digest: Sha256Digest,
    reason: Option<&FactText>,
) -> Sha256Digest {
    hasher.hash_bytes(&encode(
        SubmissionHeader {
            actor,
            case_id,
            id,
            operation_id: receipt.operation_id,
            action: receipt.action,
            expected_revision: receipt.expected_revision,
            reason,
        },
        state_digest,
    ))
}
fn encode(header: SubmissionHeader<'_>, review_digest: Sha256Digest) -> Vec<u8> {
    let mut bytes = b"DLTX1".to_vec();
    bytes.extend_from_slice(header.actor.as_uuid().as_bytes());
    bytes.extend_from_slice(header.case_id.as_uuid().as_bytes());
    bytes.extend_from_slice(header.id.as_uuid().as_bytes());
    bytes.extend_from_slice(header.operation_id.as_uuid().as_bytes());
    bytes.push(header.action.tag());
    bytes.extend_from_slice(&header.expected_revision.to_be_bytes());
    bytes.extend_from_slice(review_digest.as_bytes());
    bytes.push(u8::from(header.reason.is_some()));
    if let Some(reason) = header.reason {
        text(&mut bytes, reason.as_str());
    }
    bytes
}

pub(super) fn receipt_submission_bytes(
    actor: UserId,
    case_id: CaseId,
    id: DeadlineId,
    receipt: &DeadlineReceipt,
    reason: Option<&FactText>,
) -> Vec<u8> {
    encode(
        SubmissionHeader {
            actor,
            case_id,
            id,
            operation_id: receipt.operation_id,
            action: receipt.action,
            expected_revision: receipt.expected_revision,
            reason,
        },
        receipt.review_digest,
    )
}
