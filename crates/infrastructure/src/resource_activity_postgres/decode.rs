use super::{
    inconsistent,
    selection::{counter, digest},
    sources, stored_error,
};
use application::{
    cases::CaseActorSnapshot, procedural_facts::FactText, resource_activities::*, ApplicationError,
};
use domain::{cases::CaseId, crypto::DocumentHasher, identity::UserId};
use postgres::{Row, Transaction};
use time::OffsetDateTime;

pub(super) fn detail(
    tx: &mut Transaction<'_>,
    row: &Row,
    hasher: &dyn DocumentHasher,
) -> Result<ResourceActivityDetail, ApplicationError> {
    let case = CaseId::from_uuid(row.try_get("case_id").map_err(inconsistent)?);
    let resource = ResourceId::from_uuid(row.try_get("resource_id").map_err(inconsistent)?);
    let selection = super::selection::decode(row)?;
    let sources = sources::load(tx, case, resource, selection, hasher).map_err(stored_error)?;
    let recorded_administration =
        crate::procedural_fact_postgres::administration::captured(tx, row, case, hasher)
            .map_err(stored_error)?;
    let head_revision =
        ResourceRevision::new(counter(row, "recorded_resource_revision")?).map_err(inconsistent)?;
    let head_digest = digest(row, "recorded_resource_capture_digest")?;
    let head = crate::procedural_resource_postgres::storage::detail(
        tx,
        case,
        resource,
        Some(head_revision),
        hasher,
    )
    .map_err(stored_error)?;
    if head.receipt.capture_digest != head_digest {
        return Err(inconsistent("association recorded resource head differs"));
    }
    let revision =
        ResourceActivityRevision::new(counter(row, "revision")?).map_err(inconsistent)?;
    let action = match row
        .try_get::<_, String>("action")
        .map_err(inconsistent)?
        .as_str()
    {
        "link" => ResourceActivityAction::Link,
        "unlink" => ResourceActivityAction::Unlink,
        _ => return Err(inconsistent("association action differs")),
    };
    if action == ResourceActivityAction::Link
        && head.status != application::procedural_resources::ResourceStatus::Active
    {
        return Err(inconsistent("association link observed archived resource"));
    }
    let status = match row
        .try_get::<_, String>("status")
        .map_err(inconsistent)?
        .as_str()
    {
        "linked" => ResourceActivityStatus::Linked,
        "unlinked" => ResourceActivityStatus::Unlinked,
        _ => return Err(inconsistent("association status differs")),
    };
    let previous = if revision.get() == 1 {
        if row
            .try_get::<_, Option<Vec<u8>>>("previous_capture_digest")
            .map_err(inconsistent)?
            .is_some()
        {
            return Err(inconsistent("initial association has a predecessor"));
        }
        None
    } else {
        Some(ResourceActivityRevisionRef {
            revision: ResourceActivityRevision::new(revision.get() - 1).map_err(inconsistent)?,
            capture_digest: digest(row, "previous_capture_digest")?,
        })
    };
    let seconds: i64 = row.try_get("recorded_at_seconds").map_err(inconsistent)?;
    let nanos: i32 = row
        .try_get("recorded_at_nanoseconds")
        .map_err(inconsistent)?;
    let at = OffsetDateTime::from_unix_timestamp(seconds)
        .map_err(inconsistent)?
        .replace_nanosecond(u32::try_from(nanos).map_err(inconsistent)?)
        .map_err(inconsistent)?;
    if at < head.recorded_at {
        return Err(inconsistent("association predates its recorded head"));
    }
    let reason = row
        .try_get::<_, Option<String>>("reason")
        .map_err(inconsistent)?
        .map(|value| {
            let reason = FactText::new(&value).map_err(inconsistent)?;
            if reason.as_str() != value {
                return Err(inconsistent("association reason is not canonical"));
            }
            Ok(reason)
        })
        .transpose()?;
    let detail = ResourceActivityDetail {
        case_id: case,
        resource_id: resource,
        id: ResourceActivityId::from_uuid(row.try_get("association_id").map_err(inconsistent)?),
        revision,
        selection,
        status,
        sources,
        reason,
        receipt: ResourceActivityReceipt {
            operation_id: ResourceActivityOperationId::from_uuid(
                row.try_get("operation_id").map_err(inconsistent)?,
            ),
            action,
            expected_revision: revision.get() - 1,
            expected_resource_revision: head_revision,
            previous,
            submission_digest: digest(row, "submission_digest")?,
            capture_digest: digest(row, "capture_digest")?,
        },
        recorded_by: CaseActorSnapshot {
            id: UserId::from_uuid(row.try_get("recorded_by").map_err(inconsistent)?),
            email: row.try_get("recorded_by_email").map_err(inconsistent)?,
        },
        recorded_at: at,
        recorded_administration,
        recorded_resource_head: ResourceCaptureRef {
            id: resource,
            revision: head_revision,
            capture_digest: head_digest,
        },
    };
    resource_activity_receipt_matches(hasher, &detail)?;
    if resource_activity_submission_bytes(hasher, &draft(&detail)?)?
        != row
            .try_get::<_, Vec<u8>>("submission_canonical")
            .map_err(inconsistent)?
        || resource_activity_capture_bytes(&detail)
            != row
                .try_get::<_, Vec<u8>>("capture_canonical")
                .map_err(inconsistent)?
    {
        return Err(inconsistent("association receipt bytes differ"));
    }
    Ok(detail)
}
pub(super) fn draft(
    detail: &ResourceActivityDetail,
) -> Result<ResourceActivityDraft, ApplicationError> {
    Ok(ResourceActivityDraft {
        case_id: detail.case_id,
        resource_id: detail.resource_id,
        command: resource_activity_command_from_detail(detail)?,
        result_revision: detail.revision,
        selection: detail.selection,
        status: detail.status,
        sources: detail.sources.clone(),
        previous: detail.receipt.previous,
        recorded_by: detail.recorded_by.clone(),
        observed_administration: detail.recorded_administration.clone(),
        observed_resource_head: detail.recorded_resource_head,
        submission_digest: detail.receipt.submission_digest,
    })
}
