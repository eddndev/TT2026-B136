use super::inconsistent;
use application::{
    procedural_resources::{ResourceActId, ResourceActRevision},
    resource_activities::*,
    ApplicationError,
};
use domain::{
    crypto::Sha256Digest,
    deadlines::{DeadlineId, DeadlineRevision},
    hearings::{HearingId, HearingRevision},
};
use postgres::Row;

pub(super) fn counter(row: &Row, key: &str) -> Result<u32, ApplicationError> {
    u32::try_from(row.try_get::<_, i64>(key).map_err(inconsistent)?).map_err(inconsistent)
}
pub(super) fn digest(row: &Row, key: &str) -> Result<Sha256Digest, ApplicationError> {
    let bytes: Vec<u8> = row.try_get(key).map_err(inconsistent)?;
    Ok(Sha256Digest::from_array(bytes.try_into().map_err(
        |_| inconsistent("association digest length differs"),
    )?))
}
pub(super) fn decode(row: &Row) -> Result<ResourceActivitySelection, ApplicationError> {
    let resource = ResourceCaptureRef {
        id: ResourceId::from_uuid(row.try_get("resource_id").map_err(inconsistent)?),
        revision: ResourceRevision::new(counter(row, "resource_revision")?)
            .map_err(inconsistent)?,
        capture_digest: digest(row, "resource_capture_digest")?,
    };
    let act_id: Option<uuid::Uuid> = row.try_get("act_id").map_err(inconsistent)?;
    let act = act_id
        .map(|id| {
            Ok::<_, ApplicationError>(ResourceActCaptureRef {
                id: ResourceActId::from_uuid(id),
                revision: ResourceActRevision::new(counter(row, "act_revision")?)
                    .map_err(inconsistent)?,
                resource_revision: ResourceRevision::new(counter(row, "act_resource_revision")?)
                    .map_err(inconsistent)?,
                capture_digest: digest(row, "act_capture_digest")?,
            })
        })
        .transpose()?;
    if act.is_none()
        && (["act_revision", "act_resource_revision"]
            .iter()
            .any(|key| row.get::<_, Option<i64>>(*key).is_some())
            || row
                .get::<_, Option<Vec<u8>>>("act_capture_digest")
                .is_some())
    {
        return Err(inconsistent("association act fields are incomplete"));
    }
    let target = match row
        .try_get::<_, String>("target_kind")
        .map_err(inconsistent)?
        .as_str()
    {
        "hearing" => {
            absent_target(
                row,
                "deadline_id",
                "deadline_revision",
                "deadline_capture_digest",
            )?;
            ResourceActivityTarget::Hearing {
                id: HearingId::from_uuid(row.try_get("hearing_id").map_err(inconsistent)?),
                revision: HearingRevision::new(counter(row, "hearing_revision")?)
                    .map_err(inconsistent)?,
                submission_digest: digest(row, "hearing_submission_digest")?,
            }
        }
        "deadline" => {
            absent_target(
                row,
                "hearing_id",
                "hearing_revision",
                "hearing_submission_digest",
            )?;
            ResourceActivityTarget::Deadline {
                id: DeadlineId::from_uuid(row.try_get("deadline_id").map_err(inconsistent)?),
                revision: DeadlineRevision::new(counter(row, "deadline_revision")?)
                    .map_err(inconsistent)?,
                capture_digest: digest(row, "deadline_capture_digest")?,
            }
        }
        _ => return Err(inconsistent("association target kind differs")),
    };
    let result = ResourceActivitySelection {
        resource,
        act,
        target,
    };
    if result.canonical_bytes()
        != row
            .try_get::<_, Vec<u8>>("selection_canonical")
            .map_err(inconsistent)?
    {
        return Err(inconsistent("association selection bytes differ"));
    }
    Ok(result)
}
fn absent_target(
    row: &Row,
    id: &str,
    revision: &str,
    digest: &str,
) -> Result<(), ApplicationError> {
    if row
        .try_get::<_, Option<uuid::Uuid>>(id)
        .map_err(inconsistent)?
        .is_some()
        || row
            .try_get::<_, Option<i64>>(revision)
            .map_err(inconsistent)?
            .is_some()
        || row
            .try_get::<_, Option<Vec<u8>>>(digest)
            .map_err(inconsistent)?
            .is_some()
    {
        return Err(inconsistent("association contains another target family"));
    }
    Ok(())
}
