use super::{inconsistent, tracked_state, DeadlineTrackingCapture};
use crate::{
    cases::CurrentCaseAdministration,
    deadline_reevaluation::Observations,
    deadline_tracking::{
        DeadlineReviewState, TrackingDependency, TrackingPolicies, TrackingPolicy, TrackingReview,
        TrackingReviewReason, TrackingReviewRequirement,
    },
    ApplicationError,
};
use domain::{case_administration::CaseRevision, crypto::DocumentHasher};

type Result<T> = std::result::Result<T, ApplicationError>;

const MIN_CAPTURE_BYTES: usize = 102;
const MAX_CAPTURE_BYTES: usize = 122;

/// Parsed metadata remains incomplete until exact observations and administration
/// reproduce the original DLST2 suffix. No field can be supplied independently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadlineTrackingMetadata {
    policies: TrackingPolicies,
    review: TrackingReview,
    administration_revision: Option<CaseRevision>,
    canonical: Vec<u8>,
}

impl DeadlineTrackingMetadata {
    pub const fn administration_revision(&self) -> Option<CaseRevision> {
        self.administration_revision
    }

    /// Restore only exact committed material. The repository separately verifies
    /// observation evidence, historical receipts and the complete deadline record.
    pub fn restore(
        self,
        hasher: &dyn DocumentHasher,
        observations: Observations,
        administration: CurrentCaseAdministration,
    ) -> Result<DeadlineTrackingCapture> {
        if administration.revision() != self.administration_revision {
            return Err(inconsistent(
                "stored tracking administration revision differs",
            ));
        }
        let value = DeadlineTrackingCapture {
            policies: self.policies,
            review: self.review,
            observations,
            administration,
        };
        // The existing suffix binds DLOB1, administrative values and complete
        // administrative evidence, including the original offset and nanoseconds.
        if deadline_tracking_capture_bytes(hasher, &value)? != self.canonical {
            return Err(inconsistent("stored tracking capture commitments differ"));
        }
        Ok(value)
    }
}

/// Encode exactly the tracking suffix already present in DLST2, with no prefix.
/// This validates local shape and commitments, not persisted source authenticity.
pub fn deadline_tracking_capture_bytes(
    hasher: &dyn DocumentHasher,
    value: &DeadlineTrackingCapture,
) -> Result<Vec<u8>> {
    let digest = tracked_state::validate_capture(hasher, value)?;
    let mut bytes = Vec::with_capacity(MAX_CAPTURE_BYTES);
    tracked_state::append(&mut bytes, hasher, value, digest, true);
    Ok(bytes)
}

/// Decode the bounded DLST2 tracking suffix without inventing absent material.
/// Policies and dependency presence are checked together during restoration.
pub fn decode_deadline_tracking_capture(bytes: &[u8]) -> Result<DeadlineTrackingMetadata> {
    if !(MIN_CAPTURE_BYTES..=MAX_CAPTURE_BYTES).contains(&bytes.len()) {
        return Err(inconsistent("invalid tracking capture byte length"));
    }
    let mut reader = Reader { remaining: bytes };
    let policies = TrackingPolicies {
        profile: reader.policy()?,
        source: reader.policy()?,
        calendar: reader.policy()?,
    };
    let review = reader.review()?;
    reader.array::<32>()?;
    let administration_revision = match reader.byte()? {
        0 => None,
        1 => Some(
            CaseRevision::new(u32::from_be_bytes(reader.array()?))
                .map_err(|error| inconsistent(&error.to_string()))?,
        ),
        _ => return Err(inconsistent("invalid tracking administration presence")),
    };
    reader.array::<32>()?;
    reader.array::<32>()?;
    if !reader.remaining.is_empty() {
        return Err(inconsistent("tracking capture has trailing bytes"));
    }
    Ok(DeadlineTrackingMetadata {
        policies,
        review,
        administration_revision,
        canonical: bytes.to_vec(),
    })
}

struct Reader<'a> {
    remaining: &'a [u8],
}

impl Reader<'_> {
    fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let (value, remaining) = self
            .remaining
            .split_at_checked(N)
            .ok_or_else(|| inconsistent("truncated tracking capture"))?;
        self.remaining = remaining;
        value
            .try_into()
            .map_err(|_| inconsistent("invalid tracking capture field"))
    }

    fn byte(&mut self) -> Result<u8> {
        Ok(self.array::<1>()?[0])
    }

    fn policy(&mut self) -> Result<TrackingPolicy> {
        match self.byte()? {
            0 => Ok(TrackingPolicy::Undetermined),
            1 => Ok(TrackingPolicy::Fixed),
            2 => Ok(TrackingPolicy::Follow),
            _ => Err(inconsistent("invalid tracking policy")),
        }
    }

    fn review(&mut self) -> Result<TrackingReview> {
        let state = match self.byte()? {
            0 => DeadlineReviewState::LegacyUndeclared,
            1 => DeadlineReviewState::Accepted,
            2 => DeadlineReviewState::Pending,
            _ => return Err(inconsistent("invalid tracking review state")),
        };
        let count = self.byte()?;
        if count > 8 {
            return Err(inconsistent("too many tracking review reasons"));
        }
        let mut reasons = Vec::with_capacity(usize::from(count));
        for _ in 0..count {
            let dependency = match self.byte()? {
                0 => TrackingDependency::Profile,
                1 => TrackingDependency::Source,
                2 => TrackingDependency::Calendar,
                _ => return Err(inconsistent("invalid tracking review dependency")),
            };
            let reason = match self.byte()? {
                0 => TrackingReviewReason::SourceChanged,
                1 => TrackingReviewReason::ProfileChanged,
                2 => TrackingReviewReason::DependencyRetired,
                3 => TrackingReviewReason::PolicyUndetermined,
                _ => return Err(inconsistent("invalid tracking review reason")),
            };
            reasons.push(TrackingReviewRequirement { dependency, reason });
        }
        TrackingReview::new(state, reasons).map_err(|error| inconsistent(&error.to_string()))
    }
}
