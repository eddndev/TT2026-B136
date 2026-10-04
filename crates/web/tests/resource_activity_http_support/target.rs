use super::*;
use application::{
    deadlines::DeadlineError, hearings::HearingError, resource_activities::*, ApplicationError,
};
use domain::{cases::CaseId, crypto::Sha256Digest};
use time::{Duration, UtcOffset};
use uuid::Uuid;

pub(super) fn page(
    token: &str,
    target: ResourceActivityTargetId,
    query: ResourceActivityTargetQuery,
) -> Result<ResourceActivityTargetPage, ApplicationError> {
    match token {
        "missing_hearing" => return Err(HearingError::NotFound.into()),
        "missing_deadline" => return Err(DeadlineError::NotFound.into()),
        _ => {}
    }
    let kind = match target {
        ResourceActivityTargetId::Hearing(_) => ResourceActivityKind::Hearing,
        ResourceActivityTargetId::Deadline(_) => ResourceActivityKind::Deadline,
        ResourceActivityTargetId::ResourceHearing(_) => ResourceActivityKind::ResourceHearing,
    };
    let mut page = ResourceActivityTargetPage {
        checked_at: now(),
        associations: Vec::new(),
        has_more: false,
        next_after_id: None,
    };
    if matches!(token, "empty" | "empty_bad_time") {
        if token == "empty_bad_time" {
            page.checked_at = now().to_offset(UtcOffset::from_hms(1, 0, 0).unwrap());
        }
        return Ok(page);
    }
    let unlink = query.status() == Some(ResourceActivityStatus::Unlinked);
    let mut command = command(kind, unlink && token != "wrong_status");
    if token == "act" {
        if let ResourceActivityChange::Link { selection } = &mut command.change {
            selection.act = Some(super::act::reference());
        }
    }
    let mut row = view(&command, kind);
    match token {
        "foreign_case" => row.association.case_id = CaseId::from_uuid(FOREIGN.parse().unwrap()),
        "foreign_resource" => {
            row.association.resource_id = ResourceId::from_uuid(FOREIGN.parse().unwrap())
        }
        "wrong_target" => match &mut row.association.selection.target {
            ResourceActivityTarget::Hearing { id, .. } => {
                *id = application::hearings::HearingId::from_uuid(FOREIGN.parse().unwrap())
            }
            ResourceActivityTarget::Deadline { id, .. } => {
                *id = application::deadlines::DeadlineId::from_uuid(FOREIGN.parse().unwrap())
            }
            ResourceActivityTarget::ResourceHearing { id, .. } => {
                *id = domain::resource_hearings::ResourceHearingId::from_uuid(
                    FOREIGN.parse().unwrap(),
                )
            }
        },
        "wrong_kind" => {
            let other = match kind {
                ResourceActivityKind::Hearing => ResourceActivityKind::Deadline,
                ResourceActivityKind::Deadline => ResourceActivityKind::Hearing,
                ResourceActivityKind::ResourceHearing => ResourceActivityKind::Hearing,
            };
            row = view(&model::command(other, false), other);
        }
        "wrong_capture" => {
            row.association.sources.resource.receipt.capture_digest =
                Sha256Digest::from_array([9; 32]);
        }
        "wrong_checked_at" => row.checked_at += Duration::seconds(1),
        "wrong_current" => {
            if let ResourceActivityCurrentTarget::Hearing(current) = &mut row.current_target {
                current.snapshot.id =
                    application::hearings::HearingId::from_uuid(FOREIGN.parse().unwrap());
            }
        }
        _ => {}
    }
    page.associations.push(row.clone());
    match token {
        "duplicate" => page.associations.push(row),
        "descending" | "pagination" | "oversized" => {
            command.association_id =
                ResourceActivityId::from_uuid(Uuid::from_u128(if token == "descending" {
                    0
                } else {
                    9
                }));
            page.associations.push(view(&command, kind));
        }
        _ => {}
    }
    if matches!(token, "pagination" | "short_page") {
        page.has_more = true;
        page.next_after_id = page.associations.last().map(|v| v.association.id);
    }
    if token == "cursor_without_more" {
        page.next_after_id = Some(association());
    }
    if token == "wrong_cursor" {
        page.has_more = true;
        page.next_after_id = Some(ResourceActivityId::from_uuid(FOREIGN.parse().unwrap()));
    }
    Ok(page)
}
