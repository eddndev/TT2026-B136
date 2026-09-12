//! Reloads durable actor identity while preventing concurrent account changes.

use std::str::FromStr;

use application::identity::Principal;
use application::ApplicationError;
use domain::identity::{Role, UserId};
use postgres::Transaction;

pub(crate) fn active_actor(
    transaction: &mut Transaction<'_>,
    id: UserId,
) -> Result<Principal, ApplicationError> {
    let row = transaction
        .query_opt(
            "SELECT email,role FROM users WHERE id=$1 AND active FOR SHARE",
            &[&id.as_uuid()],
        )
        .map_err(|error| ApplicationError::Port(format!("actor database: {error}")))?
        .ok_or(ApplicationError::InvalidSession)?;
    Ok(Principal {
        id,
        email: row.get("email"),
        role: Role::from_str(row.get::<_, &str>("role"))?,
    })
}
