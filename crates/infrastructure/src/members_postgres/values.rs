use application::{
    members::{validate_user_summary, MemberError, UserSummary},
    ApplicationError,
};
use domain::identity::{Role, UserId};
use postgres::{Row, Transaction};
use std::str::FromStr;

pub(super) fn owner(tx: &mut Transaction<'_>, actor: UserId) -> Result<String, ApplicationError> {
    let principal = crate::postgres_actor::active_actor(tx, actor)?;
    if principal.role != Role::Owner {
        return Err(ApplicationError::PermissionDenied);
    }
    Ok(principal.email)
}
pub(crate) fn summary(row: &Row) -> Result<UserSummary, ApplicationError> {
    let user = UserSummary {
        id: UserId::from_uuid(row.get("id")),
        email: row.get("email"),
        role: Role::from_str(row.get::<_, &str>("role"))
            .map_err(|_| MemberError::Stored("invalid user role"))?,
        active: row.get("active"),
        revision: counter(row.get("revision"))?,
    };
    validate_user_summary(&user)?;
    Ok(user)
}
pub(super) fn counter(value: i64) -> Result<u64, ApplicationError> {
    u64::try_from(value).map_err(|_| MemberError::Stored("invalid user counter").into())
}
