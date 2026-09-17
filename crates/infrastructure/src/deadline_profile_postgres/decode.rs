use super::{header, inconsistent, projection};
use application::{deadline_profiles::*, ApplicationError};
use domain::crypto::DocumentHasher;
use postgres::Row;
pub(super) fn row(
    row: &Row,
    hasher: &dyn DocumentHasher,
) -> Result<DeadlineProfileDetail, ApplicationError> {
    let entry = header::row(row, hasher)?;
    let bytes: Vec<u8> = row.try_get("definition_canonical").map_err(inconsistent)?;
    let definition = match entry.algorithm {
        DeadlineProfileAlgorithm::V1 => {
            decode_deadline_profile_definition(&bytes).map_err(inconsistent)?
        }
    };
    if projection::definition(&definition)
        != row
            .try_get::<_, serde_json::Value>("definition_view")
            .map_err(inconsistent)?
    {
        return Err(inconsistent("profile definition differs from its summary"));
    }
    let detail = DeadlineProfileDetail {
        id: entry.id,
        revision: entry.revision,
        definition,
        definition_digest: entry.definition_digest,
        algorithm: entry.algorithm,
        status: entry.status,
        reason: entry.reason,
        receipt: entry.receipt,
        recorded_at: entry.recorded_at,
        recorded_by: entry.recorded_by,
    };
    deadline_profile_receipt_matches(hasher, &detail)?;
    Ok(detail)
}
