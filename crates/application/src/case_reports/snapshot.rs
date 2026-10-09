use super::{checks::*, *};
use crate::{cases::CaseStatusFilter, ApplicationError};
use domain::{
    case_administration::CaseAdministrativeStatus, cases::CaseMetadata, typed_participants::Uuid,
};
use std::collections::BTreeMap;

pub(super) fn validate(value: &CaseReportSnapshot) -> Result<(), ApplicationError> {
    if value.cases.len() > MAX_REPORT_CASES || value.workload.len() > MAX_REPORT_WORKLOAD {
        return Err(CaseReportError::CapacityExceeded.into());
    }
    match value.filters.kind {
        CaseReportKind::CaseState if value.activity.is_some() => {
            return Err(inconsistent("case state snapshot contains activity"));
        }
        CaseReportKind::LitigatorActivity if !value.workload.is_empty() => {
            return Err(inconsistent("activity snapshot contains case workload"));
        }
        _ => {}
    }
    let mut assignments = 0usize;
    for case in &value.cases {
        assignments = assignments
            .checked_add(case.assigned_litigators.len())
            .ok_or(CaseReportError::CapacityExceeded)?;
        if assignments > MAX_REPORT_ASSIGNMENTS {
            return Err(CaseReportError::CapacityExceeded.into());
        }
    }
    requester(&value.requester, value.scope)?;
    stored_filters(&value.filters)?;
    time(value.checked_at)?;
    if value.report_id.as_uuid().is_nil() {
        return Err(inconsistent("snapshot report identity is absent"));
    }
    let mut prior_case = None;
    let mut workload: BTreeMap<Uuid, (String, u64, u64)> = BTreeMap::new();
    for case in &value.cases {
        row(value, case)?;
        let id = case.case_id.as_uuid();
        if id.is_nil() || prior_case.is_some_and(|prior| prior >= id) {
            return Err(inconsistent("snapshot cases are not strictly ordered"));
        }
        prior_case = Some(id);
        let mut prior_user = None;
        for who in &case.assigned_litigators {
            let id = who.user_id.as_uuid();
            email(&who.email)?;
            if id.is_nil() || prior_user.is_some_and(|prior| prior >= id) {
                return Err(inconsistent(
                    "snapshot assignments are not strictly ordered",
                ));
            }
            prior_user = Some(id);
            if !workload.contains_key(&id) && workload.len() >= MAX_REPORT_WORKLOAD {
                return Err(CaseReportError::CapacityExceeded.into());
            }
            let entry = workload
                .entry(id)
                .or_insert_with(|| (who.email.clone(), 0, 0));
            if entry.0 != who.email {
                return Err(inconsistent("snapshot member email differs between cases"));
            }
            match case.status {
                CaseAdministrativeStatus::Active => entry.1 += 1,
                CaseAdministrativeStatus::Closed => entry.2 += 1,
            }
        }
    }
    if value.filters.kind == CaseReportKind::LitigatorActivity {
        return super::activity::validate(value);
    }
    if workload.len() != value.workload.len() {
        return Err(inconsistent("snapshot workload omits or invents members"));
    }
    for ((id, (email, active, closed)), actual) in workload.iter().zip(&value.workload) {
        if *id != actual.litigator.user_id.as_uuid()
            || *email != actual.litigator.email
            || *active != actual.active_cases
            || *closed != actual.closed_cases
        {
            return Err(inconsistent(
                "snapshot workload differs from captured cases",
            ));
        }
    }
    Ok(())
}
fn row(snapshot: &CaseReportSnapshot, case: &CaseReportRow) -> Result<(), ApplicationError> {
    if case.title.len() > 800 || case.reference.len() > 400 {
        return Err(inconsistent(
            "snapshot case metadata exceeds its domain limit",
        ));
    }
    let metadata = CaseMetadata::new(&case.title, &case.reference)
        .map_err(|_| inconsistent("snapshot case metadata is invalid"))?;
    if metadata.title() != case.title
        || metadata.reference() != case.reference
        || case.administration_revision.is_some() != case.administration_digest.is_some()
    {
        return Err(inconsistent(
            "snapshot case metadata or administration capture differs",
        ));
    }
    time(case.created_at)?;
    let filters = &snapshot.filters;
    let outside_state_filter = filters.kind == CaseReportKind::CaseState
        && (case.created_at < filters.period_from
            || case.created_at >= filters.period_before
            || filters
                .litigator
                .is_some_and(|id| !case.assigned_litigators.iter().any(|who| who.user_id == id)));
    if outside_state_filter
        || case.created_at > snapshot.checked_at
        || matches!(
            (filters.status, case.status),
            (CaseStatusFilter::Active, CaseAdministrativeStatus::Closed)
                | (CaseStatusFilter::Closed, CaseAdministrativeStatus::Active)
        )
        || (snapshot.scope == CaseReportScope::AssignedCases
            && !case
                .assigned_litigators
                .iter()
                .any(|who| who.user_id == snapshot.requester.principal.id))
    {
        return Err(inconsistent(
            "snapshot case is outside its captured filters or scope",
        ));
    }
    Ok(())
}
