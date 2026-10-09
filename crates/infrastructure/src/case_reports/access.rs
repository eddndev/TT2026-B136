use super::*;
use domain::identity::{Role, UserId};

pub(super) fn actor(
    tx: &mut Transaction<'_>,
    expected: &Principal,
) -> Result<CaseReportRequester, ApplicationError> {
    if !matches!(expected.role, Role::Owner | Role::Litigator) {
        return Err(ApplicationError::PermissionDenied);
    }
    let row = tx.query_opt("SELECT email,role,revision,auth_generation FROM users WHERE id=$1 AND active FOR SHARE", &[&expected.id.as_uuid()]).map_err(port)?
        .ok_or(ApplicationError::InvalidSession)?;
    let principal = Principal {
        id: expected.id,
        email: row.get("email"),
        role: row.get::<_, &str>("role").parse().map_err(inconsistent)?,
    };
    if principal != *expected {
        return Err(ApplicationError::InvalidSession);
    }
    Ok(CaseReportRequester {
        principal,
        account_revision: u64::try_from(row.get::<_, i64>("revision")).map_err(inconsistent)?,
        auth_generation: u64::try_from(row.get::<_, i64>("auth_generation"))
            .map_err(inconsistent)?,
    })
}
pub(super) fn original(
    tx: &mut Transaction<'_>,
    job: &storage::Job,
) -> Result<(), ApplicationError> {
    let saved = &job.detail.requester;
    let current = match actor(tx, &saved.principal) {
        Ok(value) => value,
        Err(ApplicationError::InvalidSession | ApplicationError::PermissionDenied) => {
            return Err(CaseReportError::AccessRevoked.into())
        }
        Err(error) => return Err(error),
    };
    // Credential consumption advances revision without changing access; see
    // docs/adr/0060-durable-authorized-case-reports.md.
    if current.principal != saved.principal
        || current.auth_generation != saved.auth_generation
        || current.account_revision < saved.account_revision
        || job.detail.scope
            != if current.principal.role == Role::Owner {
                CaseReportScope::Office
            } else {
                CaseReportScope::AssignedCases
            }
    {
        return Err(CaseReportError::AccessRevoked.into());
    }
    if job.detail.scope == CaseReportScope::AssignedCases {
        let rows = tx.query("SELECT case_id FROM case_memberships WHERE user_id=$1 AND case_id=ANY($2::uuid[]) ORDER BY case_id FOR SHARE", &[&saved.principal.id.as_uuid(), &job.case_ids]).map_err(port)?;
        let ids: Vec<Uuid> = rows.iter().map(|row| row.get(0)).collect();
        if ids != job.case_ids {
            return Err(CaseReportError::AccessRevoked.into());
        }
    }
    let count: i64 = tx
        .query_one(
            "SELECT count(*) FROM cases WHERE id=ANY($1::uuid[])",
            &[&job.case_ids],
        )
        .map_err(port)?
        .get(0);
    if count != job.case_ids.len() as i64 {
        return Err(CaseReportError::AccessRevoked.into());
    }
    Ok(())
}
pub(super) const LITIGATOR_VISIBILITY: &str = "u.active AND u.role='litigator'
    AND ($1::boolean OR EXISTS(
        SELECT 1 FROM case_memberships own JOIN case_memberships other USING(case_id)
        WHERE own.user_id=$2 AND other.user_id=u.id))";

pub(super) fn filter_member(
    tx: &mut Transaction<'_>,
    actor: &Principal,
    selected: Option<UserId>,
    kind: CaseReportKind,
) -> Result<(), ApplicationError> {
    let Some(id) = selected else {
        return Ok(());
    };
    let visible = litigator_visibility(kind);
    let sql = format!(
        "SELECT u.id FROM users u WHERE {visible}
        AND u.id=$3 FOR SHARE OF u"
    );
    let visible = tx
        .query_opt(
            &sql,
            &[
                &(actor.role == Role::Owner),
                &actor.id.as_uuid(),
                &id.as_uuid(),
            ],
        )
        .map_err(port)?;
    if visible.is_none() {
        return Err(ApplicationError::InvalidInput(
            "selected report member is not an active visible litigator".into(),
        ));
    }
    Ok(())
}

pub(super) fn litigator_visibility(kind: CaseReportKind) -> String {
    if kind == CaseReportKind::CaseState {
        return LITIGATOR_VISIBILITY.into();
    }
    format!(
        "u.active AND u.role='litigator' AND ($1::boolean OR EXISTS(
        SELECT 1 FROM case_memberships own JOIN case_memberships other USING(case_id)
        WHERE own.user_id=$2 AND other.user_id=u.id) OR EXISTS(
        SELECT 1 FROM ({}) activity JOIN case_memberships own ON own.case_id=activity.case_id
        WHERE own.user_id=$2 AND activity.actor_id=u.id))",
        activity_sources::EVENTS
    )
}
