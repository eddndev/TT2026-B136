use super::*;
use crate::ApplicationError;
use domain::{
    clock::OffsetDateTime,
    crypto::{DocumentHasher, Sha256Digest},
};
use std::sync::Arc;

/// Construction is restricted to the service after exact-source and batch checks.
pub struct PreparedResourceChange {
    pub(super) material: ResourceMaterial,
    pub(super) draft: ResourceDraft,
    pub(super) hasher: Arc<dyn DocumentHasher + Send + Sync>,
}
impl PreparedResourceChange {
    pub fn command(&self) -> &ResourceCommand {
        &self.draft.command
    }
    pub fn material(&self) -> &ResourceMaterial {
        &self.material
    }
    pub fn draft(&self) -> &ResourceDraft {
        &self.draft
    }
    /// Called by the store only after rechecking the observations under its lock.
    /// The definitive timestamp comes from the store Clock inside that transaction.
    pub fn into_detail(
        self,
        recorded_at: OffsetDateTime,
    ) -> Result<ResourceDetail, ApplicationError> {
        if recorded_at.offset() != time::UtcOffset::UTC
            || !(1..=9999).contains(&recorded_at.year())
            || self
                .material
                .base
                .as_ref()
                .is_some_and(|b| recorded_at < b.recorded_at)
            || self
                .material
                .administration
                .snapshot()
                .is_some_and(|a| recorded_at < a.changed_at)
            || self
                .material
                .stage
                .entry()
                .is_some_and(|s| recorded_at < s.recorded_at())
        {
            return Err(inconsistent(
                "resource commit clock predates captured evidence",
            ));
        }
        let draft = self.draft;
        let receipt = ResourceReceipt {
            operation_id: draft.command.operation_id,
            action: draft.command.action(),
            expected_revision: draft.command.expected_revision(),
            previous: draft.previous,
            values_digest: self.hasher.hash_bytes(&draft.values.canonical_bytes()),
            sources_digest: self
                .hasher
                .hash_bytes(&resource_sources_bytes(&draft.sources)?),
            submission_digest: draft.submission_digest,
            capture_digest: Sha256Digest::from_array([0; 32]),
        };
        let mut detail = ResourceDetail {
            case_id: draft.case_id,
            id: draft.command.resource_id,
            revision: draft.result_revision,
            values: draft.values,
            status: draft.status,
            sources: draft.sources,
            act: draft.act,
            reason: draft.command.reason().cloned(),
            receipt,
            recorded_by: draft.recorded_by,
            recorded_at,
            recorded_administration: draft.observed_administration,
            recorded_stage: draft.observed_stage,
        };
        detail.receipt.capture_digest = self.hasher.hash_bytes(&resource_capture_bytes(&detail));
        resource_receipt_matches(self.hasher.as_ref(), &detail)?;
        Ok(detail)
    }
}
