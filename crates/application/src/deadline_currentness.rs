//! Current operational views over immutable deadline evidence.

use crate::{
    deadline_observations::build_deadline_observations,
    deadline_reevaluation::ObservationRole,
    deadline_technical::{policy, validation, DeadlineReevaluationInputs},
    deadline_tracking::{
        decide_tracking_change, DeadlineReviewState, TrackingChange, TrackingDependency,
        TrackingDisposition,
    },
    deadlines::{
        deadline_receipt_matches, DeadlineAttention, DeadlineDetail, DeadlineError, DeadlineId,
        DeadlineOverview, DeadlineReceiptKind, DeadlineResponsibleSnapshot, DeadlineRevision,
        DeadlineStatus,
    },
    ApplicationError,
};
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::{DocumentHasher, Sha256Digest},
    procedural_facts::FactLabel,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlineFreshness {
    Current,
    Changed,
    NotChecked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineOperational {
    binding: CaptureBinding,
    freshness: DeadlineFreshness,
    requires_review: bool,
    checked_at: Option<OffsetDateTime>,
    changed_dependencies: Vec<TrackingDependency>,
    due_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CaptureBinding {
    case_id: CaseId,
    id: DeadlineId,
    revision: DeadlineRevision,
    capture_digest: Sha256Digest,
    receipt_kind: DeadlineReceiptKind,
    title: FactLabel,
    responsible: DeadlineResponsibleSnapshot,
    attention_recorded: bool,
    status: DeadlineStatus,
    review: DeadlineReviewState,
    calculation_due_at: Option<OffsetDateTime>,
}

impl CaptureBinding {
    fn new(detail: &DeadlineDetail) -> Self {
        Self {
            case_id: detail.case_id,
            id: detail.id,
            revision: detail.revision,
            capture_digest: detail.receipt.capture_digest,
            receipt_kind: DeadlineReceiptKind::from(&detail.receipt.version),
            title: detail.definition.title.clone(),
            responsible: detail.responsible.clone(),
            attention_recorded: matches!(detail.attention, DeadlineAttention::Recorded { .. }),
            status: detail.status,
            review: detail.review_state(),
            calculation_due_at: detail.calculation.result.due_at(),
        }
    }
}

impl DeadlineOperational {
    /// Check every compact field against the verified capture that produced this view.
    pub fn matches_overview(&self, overview: &DeadlineOverview) -> bool {
        self.binding.case_id == overview.case_id
            && self.binding.id == overview.id
            && self.binding.revision == overview.revision
            && self.binding.capture_digest == overview.capture_digest()
            && self.binding.receipt_kind == overview.receipt_kind
            && self.binding.title == overview.title
            && self.binding.responsible == overview.responsible
            && self.binding.attention_recorded == overview.attention_recorded
            && self.binding.status == overview.status
            && self.binding.review == overview.review_state
            && self.binding.calculation_due_at == overview.calculation_due_at
            && self.binding.calculation_due_at.map(|value| value.offset())
                == overview.calculation_due_at.map(|value| value.offset())
            && overview.calculation_blocked == overview.calculation_due_at.is_none()
    }

    /// Check the original capture and summary binding; this does not verify a receipt.
    pub fn matches_capture(&self, detail: &DeadlineDetail) -> bool {
        self.binding == CaptureBinding::new(detail)
            && self.binding.calculation_due_at.map(|value| value.offset())
                == detail
                    .calculation
                    .result
                    .due_at()
                    .map(|value| value.offset())
    }

    /// Whether verified current evidence calls for human applicability review.
    pub const fn requires_review(&self) -> bool {
        self.requires_review
    }

    pub const fn freshness(&self) -> DeadlineFreshness {
        self.freshness
    }

    pub const fn checked_at(&self) -> Option<OffsetDateTime> {
        self.checked_at
    }

    pub fn changed_dependencies(&self) -> &[TrackingDependency] {
        &self.changed_dependencies
    }

    pub const fn due_at(&self) -> Option<OffsetDateTime> {
        self.due_at
    }
}

/// The operational view is bound to the exact immutable record examined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineCurrent {
    detail: DeadlineDetail,
    operational: DeadlineOperational,
}

impl DeadlineCurrent {
    /// Verify captured evidence without making a claim about current dependency heads.
    /// Storage additionally verifies the exact historical observation references.
    pub fn historical(
        hasher: &dyn DocumentHasher,
        detail: &DeadlineDetail,
    ) -> Result<Self, ApplicationError> {
        deadline_receipt_matches(hasher, detail)?;
        Ok(Self::unchecked(detail))
    }

    pub const fn detail(&self) -> &DeadlineDetail {
        &self.detail
    }

    pub const fn operational(&self) -> &DeadlineOperational {
        &self.operational
    }

    pub fn into_parts(self) -> (DeadlineDetail, DeadlineOperational) {
        (self.detail, self.operational)
    }

    fn unchecked(detail: &DeadlineDetail) -> Self {
        Self {
            detail: detail.clone(),
            operational: DeadlineOperational {
                binding: CaptureBinding::new(detail),
                freshness: DeadlineFreshness::NotChecked,
                requires_review: false,
                checked_at: None,
                changed_dependencies: Vec::new(),
                due_at: None,
            },
        }
    }
}

/// Verify current dependency evidence without mutating the captured record.
/// Only legacy or retired records may omit current inputs. The caller resolves
/// the actual heads together under authorized storage access before invoking this.
pub fn evaluate_deadline_currentness(
    hasher: &dyn DocumentHasher,
    base: &DeadlineDetail,
    inputs: Option<&DeadlineReevaluationInputs>,
    checked_at: OffsetDateTime,
) -> Result<DeadlineCurrent, ApplicationError> {
    deadline_receipt_matches(hasher, base)?;
    if base.status == DeadlineStatus::Retired
        || base.review_state() == DeadlineReviewState::LegacyUndeclared
    {
        return Ok(DeadlineCurrent::unchecked(base));
    }
    let inputs = inputs.ok_or_else(|| inconsistent("current dependency evidence is absent"))?;
    let tracking = base
        .tracking
        .as_ref()
        .ok_or_else(|| inconsistent("current deadline tracking is absent"))?;
    validation::selected(hasher, base, inputs)?;
    let observations = build_deadline_observations(
        hasher,
        base.case_id,
        &inputs.profile_head,
        &inputs.material,
        inputs.notification_parent_head.as_ref(),
    )?;
    if tracking.observations.entries.len() != observations.entries.len() {
        return Err(inconsistent(
            "current observation roles differ from the capture",
        ));
    }
    validation::advance(&tracking.observations, &observations)?;
    let mut changed_dependencies = Vec::new();
    let mut requires_review = base.review_state() == DeadlineReviewState::Pending;
    for entry in &observations.entries {
        let (dependency, declared_policy) = match entry.role {
            ObservationRole::Profile => (TrackingDependency::Profile, tracking.policies.profile),
            ObservationRole::Source | ObservationRole::NotificationParent => {
                (TrackingDependency::Source, tracking.policies.source)
            }
            ObservationRole::Calendar => (TrackingDependency::Calendar, tracking.policies.calendar),
        };
        let previous = tracking
            .observations
            .entries
            .iter()
            .find(|value| value.role == entry.role)
            .ok_or_else(|| inconsistent("current dependency was not captured"))?;
        let retired = policy::retired(inputs, entry.role);
        if retired
            && entry.revision == previous.revision
            && base.review_state() == DeadlineReviewState::Accepted
        {
            return Err(inconsistent(
                "accepted capture already observed a retired dependency",
            ));
        }
        let decision = decide_tracking_change(TrackingChange {
            dependency,
            policy: declared_policy,
            selected_revision: policy::selected_revision(base, entry.role)?,
            observed_revision: previous.revision,
            new_revision: entry.revision,
            retired,
        })
        .map_err(inconsistent)?;
        requires_review |= matches!(decision.disposition, TrackingDisposition::ReviewRequired(_));
        if matches!(
            decision.disposition,
            TrackingDisposition::Recalculate | TrackingDisposition::ReviewRequired(_)
        ) {
            changed_dependencies.push(dependency);
        }
    }
    changed_dependencies.sort_by_key(|dependency| match dependency {
        TrackingDependency::Profile => 0,
        TrackingDependency::Source => 1,
        TrackingDependency::Calendar => 2,
    });
    changed_dependencies.dedup();
    let freshness = if changed_dependencies.is_empty() {
        DeadlineFreshness::Current
    } else {
        DeadlineFreshness::Changed
    };
    Ok(DeadlineCurrent {
        detail: base.clone(),
        operational: DeadlineOperational {
            binding: CaptureBinding::new(base),
            freshness,
            requires_review,
            checked_at: Some(checked_at),
            changed_dependencies,
            due_at: if freshness == DeadlineFreshness::Current {
                base.operational_due_at()
            } else {
                None
            },
        },
    })
}

fn inconsistent(message: impl std::fmt::Display) -> ApplicationError {
    DeadlineError::StoredInconsistent(message.to_string()).into()
}
