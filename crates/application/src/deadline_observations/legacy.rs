use super::{entries, inconsistent, validation};
use crate::{
    deadline_reevaluation::{encode_observations, ObservationRole, Observations},
    deadlines::{deadline_receipt_matches, DeadlineDetail, DeadlineReceiptVersion},
    ApplicationError,
};
use domain::crypto::DocumentHasher;

/// Reconstruct only observations already bound by a verified legacy capture.
///
/// V1 preserved the selected profile and the then-observed source/calendar heads,
/// but no separate notification parent head. Its partial parent projection cannot
/// supply that missing full evidence. This function adds no parent observation,
/// does not query current heads and does not declare policies or acceptance.
pub fn build_legacy_deadline_observations(
    hasher: &dyn DocumentHasher,
    detail: &DeadlineDetail,
) -> Result<Observations, ApplicationError> {
    if !matches!(&detail.receipt.version, DeadlineReceiptVersion::Legacy)
        || detail.tracking.is_some()
    {
        return Err(inconsistent(
            "legacy observations require an untracked legacy receipt",
        ));
    }
    deadline_receipt_matches(hasher, detail)?;
    let material = &detail.calculation.material;
    validation::material(hasher, detail.case_id, material)?;
    let mut entries = vec![entries::profile(hasher, &detail.calculation.profile)];
    if let Some(source) = &material.source_head {
        entries.push(entries::source(hasher, ObservationRole::Source, source));
    }
    if let Some(calendar) = &material.calendar_head {
        entries.push(entries::calendar(hasher, calendar));
    }
    let observations = Observations {
        case_id: detail.case_id,
        entries,
    };
    encode_observations(&observations).map_err(inconsistent)?;
    Ok(observations)
}
