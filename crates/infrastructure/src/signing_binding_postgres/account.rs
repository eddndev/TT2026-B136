use super::{decode::value, inconsistent, port};
use application::{
    document_signing::SigningAccountContext,
    identity::Principal,
    members::{validate_user_summary, UserSummary},
    ApplicationError,
};
use domain::{
    document_signing::{SignerAccount, SigningError},
    identity::{Role, UserId},
};
use postgres::GenericClient;
use std::str::FromStr;

type Admit = fn(UserId, Role, bool, u64, u64) -> Result<SignerAccount, SigningError>;

/// Loads an account that may sign a document individually: an active Litigator.
/// Requires the common audit lock before acquiring the account row lock.
/// The returned trust is absent until the caller selects the required snapshot.
pub(crate) fn account<C: GenericClient>(
    client: &mut C,
    actor: UserId,
) -> Result<SigningAccountContext, ApplicationError> {
    load(client, actor, SignerAccount::new)
}

/// Loads an account that may hold a document-signing binding: any party of a
/// bilateral contract (an active Owner, Litigator or Client), as
/// docs/adr/0086-bilateral-contract-signing.md fixes. The same lock rules apply.
pub(crate) fn binding_account<C: GenericClient>(
    client: &mut C,
    actor: UserId,
) -> Result<SigningAccountContext, ApplicationError> {
    load(client, actor, SignerAccount::for_contract)
}

fn load<C: GenericClient>(
    client: &mut C,
    actor: UserId,
    admit: Admit,
) -> Result<SigningAccountContext, ApplicationError> {
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
    let revision = counter(value(&row, "revision")?)?;
    let generation = counter(value(&row, "auth_generation")?)?;
    let account =
        admit(actor, role, active, revision, generation).map_err(|error| match error {
            SigningError::LitigatorRequired | SigningError::ContractSignerRequired => {
                ApplicationError::PermissionDenied
            }
            _ => inconsistent(),
        })?;
    let email: String = value(&row, "email")?;
    validate_user_summary(&UserSummary {
        id: actor,
        email: email.clone(),
        role,
        active,
        revision,
    })
    .map_err(|_| inconsistent())?;
    Ok(SigningAccountContext {
        principal: Principal {
            id: actor,
            email,
            role,
        },
        account,
        trust: None,
    })
}
pub(super) fn counter(value: i64) -> Result<u64, ApplicationError> {
    u64::try_from(value).map_err(|_| inconsistent())
}
