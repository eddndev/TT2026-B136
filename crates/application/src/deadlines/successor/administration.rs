use super::super::*;
use crate::{cases::CurrentCaseAdministration, ApplicationError};
use domain::crypto::DocumentHasher;

pub(super) fn validate(
    hasher: &dyn DocumentHasher,
    previous: &DeadlineDetail,
    next: &DeadlineDetail,
) -> Result<(), ApplicationError> {
    let Some(next) = &next.tracking else {
        return Ok(());
    };
    let previous = previous
        .tracking
        .as_ref()
        .map_or(&previous.calculation.material.administration, |value| {
            &value.administration
        });
    validate_capture(hasher, previous, &next.administration)
}

pub(crate) fn validate_capture(
    hasher: &dyn DocumentHasher,
    previous: &CurrentCaseAdministration,
    next: &CurrentCaseAdministration,
) -> Result<(), ApplicationError> {
    let valid = match (previous.revision(), next.revision()) {
        (None, Some(_)) => true,
        (Some(_), None) => false,
        (Some(before), Some(after)) if after > before => true,
        (before, after) if before == after => {
            previous == next && bytes(hasher, previous) == bytes(hasher, next)
        }
        _ => false,
    };
    if !valid {
        return Err(inconsistent(
            "observed administration regressed or changed an immutable capture",
        ));
    }
    Ok(())
}
fn bytes(hasher: &dyn DocumentHasher, value: &CurrentCaseAdministration) -> Vec<u8> {
    let mut bytes = Vec::new();
    evidence::administration(&mut bytes, hasher, value);
    bytes
}
