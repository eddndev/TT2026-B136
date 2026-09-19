use super::port;
use application::{agenda::*, identity::Principal, ApplicationError};
use domain::{
    cases::CaseId,
    crypto::DocumentHasher,
    identity::{Permission, Role, UserId},
};
use postgres::Transaction;

pub(super) fn actor(
    tx: &mut Transaction<'_>,
    actor: UserId,
    kind: AgendaKind,
) -> Result<Principal, ApplicationError> {
    let principal = crate::postgres_actor::active_actor(tx, actor)?;
    let allowed = match kind {
        AgendaKind::All => {
            principal.role.allows(Permission::ReadHearing)
                && principal.role.allows(Permission::ReadDeadline)
        }
        AgendaKind::Hearing => principal.role.allows(Permission::ReadHearing),
        AgendaKind::Deadline => principal.role.allows(Permission::ReadDeadline),
    };
    if !allowed {
        return Err(ApplicationError::PermissionDenied);
    }
    Ok(principal)
}

pub(super) fn case_summary(
    tx: &mut Transaction<'_>,
    principal: &Principal,
    case: CaseId,
    hasher: &dyn DocumentHasher,
) -> Result<AgendaCaseSummary, ApplicationError> {
    let row = if principal.role == Role::Owner {
        tx.query_opt("SELECT id FROM cases WHERE id=$1", &[&case.as_uuid()])
    } else {
        tx.query_opt(
            "SELECT case_id FROM case_memberships WHERE case_id=$1 AND user_id=$2 FOR SHARE",
            &[&case.as_uuid(), &principal.id.as_uuid()],
        )
    }
    .map_err(port)?;
    row.ok_or(ApplicationError::CaseNotFound)?;
    let current = crate::cases::storage::detail(tx, case, hasher)?
        .administration
        .values();
    Ok(AgendaCaseSummary {
        case_id: case,
        title: current.metadata().title().into(),
        reference: current.metadata().reference().into(),
        status: current.status(),
    })
}
