use super::*;
use crate::ApplicationError;
use domain::{
    clock::OffsetDateTime,
    crypto::{DocumentHasher, Sha256Digest},
};
use std::sync::Arc;
/// Constructed only after the service verifies every exact selected source.
pub struct PreparedResourceActivityChange {
    pub(super) material: ResourceActivityMaterial,
    pub(super) draft: ResourceActivityDraft,
    pub(super) hasher: Arc<dyn DocumentHasher + Send + Sync>,
}
impl PreparedResourceActivityChange {
    pub fn command(&self) -> &ResourceActivityCommand {
        &self.draft.command
    }
    pub fn material(&self) -> &ResourceActivityMaterial {
        &self.material
    }
    pub fn draft(&self) -> &ResourceActivityDraft {
        &self.draft
    }
    pub fn into_detail(
        self,
        recorded_at: OffsetDateTime,
    ) -> Result<ResourceActivityDetail, ApplicationError> {
        super::validation::valid_time(recorded_at)?;
        if recorded_at < self.material.resource_head.recorded_at
            || self
                .material
                .base
                .as_ref()
                .is_some_and(|v| recorded_at < v.recorded_at)
        {
            return Err(inconsistent("association commit clock predates its base"));
        }
        let draft = self.draft;
        let mut detail = ResourceActivityDetail {
            case_id: draft.case_id,
            resource_id: draft.resource_id,
            id: draft.command.association_id,
            revision: draft.result_revision,
            selection: draft.selection,
            status: draft.status,
            sources: draft.sources,
            reason: draft.command.reason().cloned(),
            receipt: ResourceActivityReceipt {
                operation_id: draft.command.operation_id,
                action: draft.command.action(),
                expected_revision: draft.command.expected_revision(),
                expected_resource_revision: draft.command.expected_resource_revision,
                previous: draft.previous,
                submission_digest: draft.submission_digest,
                capture_digest: Sha256Digest::from_array([0; 32]),
            },
            recorded_by: draft.recorded_by,
            recorded_at,
            recorded_administration: draft.observed_administration,
            recorded_resource_head: draft.observed_resource_head,
        };
        detail.receipt.capture_digest = self
            .hasher
            .hash_bytes(&resource_activity_capture_bytes(&detail));
        resource_activity_receipt_matches(self.hasher.as_ref(), &detail)?;
        Ok(detail)
    }
}
