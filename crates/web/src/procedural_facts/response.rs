use super::{projection, request, sources, values};
use crate::error::ApiError;
use application::procedural_facts::*;
use domain::cases::CaseId;
use serde_json::{json, Value};

pub(super) fn draft(
    row: FactDraft,
    case: CaseId,
    expected: &ProceduralFactCommand,
) -> Result<Value, ApiError> {
    if row.case_id != case
        || row.command != *expected
        || row.result_revision
            != expected
                .result_revision()
                .map_err(|_| ApiError::internal())?
    {
        return Err(ApiError::internal());
    }
    projection::validate_values_target(&row.values, expected.target())?;
    let command_values = match expected {
        ProceduralFactCommand::Resolution(c) => c
            .change()
            .values()
            .map(|v| ProceduralFactValues::Resolution(Box::new(v.clone()))),
        ProceduralFactCommand::Notification(c) => c
            .change()
            .values()
            .map(|v| ProceduralFactValues::Notification(Box::new(v.clone()))),
    };
    if command_values.is_some_and(|v| v != row.values) {
        return Err(ApiError::internal());
    }
    sources::validate(case, &row.values, &row.sources)?;
    Ok(
        json!({"case_id":case,"actor_id":row.actor,"command":request::project(&row.command)?,
            "result_revision":row.result_revision.get(),"values":projection::project_values(&row.values)?,
            "values_digest":row.values_digest.to_hex(),"sources":sources::project(&row.sources)?,
            "sources_digest":row.sources_digest.to_hex(),"submission_digest":row.submission_digest.to_hex(),
            "observed_administration":projection::administration(&row.observed_administration,case)?
        }),
    )
}

pub(super) fn resolutions(row: ResolutionPage, case: CaseId) -> Result<Value, ApiError> {
    let ids = row
        .resolutions
        .iter()
        .map(|r| r.root.id().as_uuid())
        .collect::<Vec<_>>();
    validate_page(
        &ids,
        row.has_more,
        row.next_after_id.map(|id| id.as_uuid()),
        100,
    )?;
    if row.resolutions.iter().any(|r| r.root.case_id() != case) {
        return Err(ApiError::internal());
    }
    let items = row
        .resolutions
        .iter()
        .map(|r| {
            let mut value = projection::target(case, FactTarget::Resolution(r.root.id()));
            projection::merge(
                &mut value,
                json!({"revision":r.revision.get(),"status":projection::status(r.status),
            "class":values::class(&r.class),"issued_at":values::time(r.issued_at)?}),
            );
            Ok(value)
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    Ok(
        json!({"resolutions":items,"has_more":row.has_more,"next_after_id":row.next_after_id.map(|id|id.to_string())}),
    )
}

pub(super) fn notifications(
    row: NotificationPage,
    case: CaseId,
    parent: ResolutionId,
) -> Result<Value, ApiError> {
    let ids = row
        .notifications
        .iter()
        .map(|r| r.root.id().as_uuid())
        .collect::<Vec<_>>();
    validate_page(
        &ids,
        row.has_more,
        row.next_after_id.map(|id| id.as_uuid()),
        100,
    )?;
    if row.notifications.iter().any(|r| {
        r.root.case_id() != case || r.root.resolution_id() != parent || r.resolution.id != parent
    }) {
        return Err(ApiError::internal());
    }
    let items = row
        .notifications
        .iter()
        .map(|r| {
            let mut value = projection::target(
                case,
                FactTarget::Notification {
                    id: r.root.id(),
                    resolution_id: parent,
                },
            );
            projection::merge(
                &mut value,
                json!({"revision":r.revision.get(),"status":projection::status(r.status),
            "resolution":{"id":r.resolution.id.to_string(),"revision":r.resolution.revision.get()},
            "outcome":values::outcome(&r.outcome),"practiced_at":values::time(r.practiced_at)?}),
            );
            Ok(value)
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    Ok(
        json!({"notifications":items,"has_more":row.has_more,"next_after_id":row.next_after_id.map(|id|id.to_string())}),
    )
}

pub(super) fn history(
    row: FactHistoryPage,
    case: CaseId,
    target: FactTarget,
) -> Result<Value, ApiError> {
    let revisions = row
        .revisions
        .iter()
        .map(|r| r.metadata.revision)
        .collect::<Vec<_>>();
    if revisions.len() > 20
        || revisions.windows(2).any(|pair| pair[0] <= pair[1])
        || !cursor_matches(&revisions, row.has_more, row.next_before_revision)
    {
        return Err(ApiError::internal());
    }
    let items = row
        .revisions
        .iter()
        .map(|r| projection::history_entry(r, case, target))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(
        json!({"revisions":items,"has_more":row.has_more,"next_before_revision":row.next_before_revision.map(|r|r.get())}),
    )
}
fn validate_page<T: Ord + Copy>(
    ids: &[T],
    has_more: bool,
    next: Option<T>,
    maximum: usize,
) -> Result<(), ApiError> {
    if ids.len() > maximum
        || ids.windows(2).any(|pair| pair[0] >= pair[1])
        || !cursor_matches(ids, has_more, next)
    {
        return Err(ApiError::internal());
    }
    Ok(())
}
fn cursor_matches<T: Eq + Copy>(items: &[T], has_more: bool, next: Option<T>) -> bool {
    if has_more {
        !items.is_empty() && next == items.last().copied()
    } else {
        next.is_none()
    }
}
