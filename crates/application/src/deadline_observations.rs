//! Verified compact observations of exact deadline dependency heads.
//!
//! Building a commitment does not establish authorization, currentness in storage
//! or legal applicability. A persistence adapter resolves all material together
//! and checks each historical reference before storing or returning its digest.

mod entries;
mod legacy;
mod validation;

pub use legacy::build_legacy_deadline_observations;

use crate::{
    deadline_inputs::{DeadlineInputError, DeadlineInputMaterial, DeadlineSourceDetail},
    deadline_profiles::{
        deadline_profile_receipt_matches, DeadlineProfileDetail, DeadlineProfileScope,
    },
    deadline_reevaluation::{encode_observations, ObservationRole, Observations},
    procedural_facts::FactDetail,
    ApplicationError,
};
use domain::{cases::CaseId, crypto::DocumentHasher};

/// Observe verified heads while preserving every selected historical input.
///
/// Retired dependencies and closed cases remain observable. This function neither
/// evaluates a new trigger nor transfers an old qualification to a new profile.
/// A notification requires a separately verified parent head; the exact parent
/// inside each notification keeps its original revision and captured projection.
/// The caller binds the observed profile root to the selected deadline definition.
pub fn build_deadline_observations(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    profile_head: &DeadlineProfileDetail,
    material: &DeadlineInputMaterial,
    notification_parent_head: Option<&FactDetail>,
) -> Result<Observations, ApplicationError> {
    deadline_profile_receipt_matches(hasher, profile_head)?;
    if matches!(profile_head.definition.scope(), DeadlineProfileScope::Case(id) if *id != case_id) {
        return Err(inconsistent("observed profile belongs to another case"));
    }
    validation::material(hasher, case_id, material)?;
    validation::parent(hasher, case_id, material, notification_parent_head)?;
    let mut entries = vec![entries::profile(hasher, profile_head)];
    if let Some(source) = &material.source_head {
        entries.push(entries::source(hasher, ObservationRole::Source, source));
    }
    if let Some(calendar) = &material.calendar_head {
        entries.push(entries::calendar(hasher, calendar));
    }
    if let Some(parent) = notification_parent_head {
        let source = DeadlineSourceDetail::Fact(Box::new(parent.clone()));
        entries.push(entries::source(
            hasher,
            ObservationRole::NotificationParent,
            &source,
        ));
    }
    let observations = Observations { case_id, entries };
    encode_observations(&observations).map_err(inconsistent)?;
    Ok(observations)
}

fn inconsistent(message: impl std::fmt::Display) -> ApplicationError {
    DeadlineInputError::Inconsistent(message.to_string()).into()
}
