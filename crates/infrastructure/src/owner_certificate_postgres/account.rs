use std::str::FromStr;

use application::{
    identity::{owner_certificates::OwnerBindingAccount, Principal},
    members::{validate_user_summary, UserSummary},
    ApplicationError,
};
use domain::{
    identity::{Role, UserId},
    owner_certificates::OwnerAccount,
};
use postgres::GenericClient;

use super::{decode::value, inconsistent, port};

pub(super) fn owner<C: GenericClient>(
    client: &mut C,
    actor: UserId,
) -> Result<OwnerBindingAccount, ApplicationError> {
    let row = client
        .query_opt(
            "SELECT id,email,role,active,revision,auth_generation
        FROM users WHERE id=$1 FOR UPDATE",
            &[&actor.as_uuid()],
        )
        .map_err(port)?
        .ok_or(ApplicationError::PermissionDenied)?;
    let role = Role::from_str(value::<&str>(&row, "role")?).map_err(|_| inconsistent())?;
    let active: bool = value(&row, "active")?;
    if !active || role != Role::Owner {
        return Err(ApplicationError::PermissionDenied);
    }
    let revision = counter(value(&row, "revision")?)?;
    let generation = counter(value(&row, "auth_generation")?)?;
    let email: String = value(&row, "email")?;
    OwnerAccount::new(actor, role, active, revision, generation).map_err(|_| inconsistent())?;
    validate_user_summary(&UserSummary {
        id: actor,
        email: email.clone(),
        role,
        active,
        revision,
    })
    .map_err(|_| inconsistent())?;
    Ok(OwnerBindingAccount {
        principal: Principal {
            id: actor,
            email,
            role,
        },
        active,
        revision,
        auth_generation: generation,
    })
}

pub(super) fn counter(value: i64) -> Result<u64, ApplicationError> {
    u64::try_from(value).map_err(|_| inconsistent())
}

pub(super) fn captured(
    id: UserId,
    revision: i64,
    generation: i64,
) -> Result<OwnerAccount, ApplicationError> {
    OwnerAccount::new(
        id,
        Role::Owner,
        true,
        counter(revision)?,
        counter(generation)?,
    )
    .map_err(|_| inconsistent())
}
