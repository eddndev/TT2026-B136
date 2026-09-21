use super::{
    port,
    values::{counter, owner, summary},
    PostgresMemberStore,
};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{
    members::{MemberError, UserAccessChange, UserSummary},
    ApplicationError,
};
use domain::identity::{Role, UserId};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

impl PostgresMemberStore {
    pub(super) fn change(
        &self,
        actor: UserId,
        id: UserId,
        change: UserAccessChange,
        lower: OffsetDateTime,
    ) -> Result<UserSummary, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let email = owner(&mut tx, actor)?;
        let at = self.checked_at(lower)?;
        let row = tx.query_opt("SELECT id,email,role,active,revision,auth_generation FROM users WHERE id=$1 FOR UPDATE",
            &[&id.as_uuid()]).map_err(port)?.ok_or(ApplicationError::UserNotFound)?;
        let old = summary(&row)?;
        let generation = counter(row.get("auth_generation"))?;
        if old.revision != change.expected_revision() {
            return Err(MemberError::RevisionConflict.into());
        }
        if old.role == change.role() && old.active == change.active() {
            tx.commit().map_err(port)?;
            return Ok(old);
        }
        if old.active
            && old.role == Role::Owner
            && (!change.active() || change.role() != Role::Owner)
        {
            let another: bool = tx
                .query_one(
                    "SELECT EXISTS(SELECT 1 FROM users WHERE id<>$1 AND active AND role='owner')",
                    &[&id.as_uuid()],
                )
                .map_err(port)?
                .get(0);
            if !another {
                return Err(MemberError::LastActiveOwner.into());
            }
        }
        if old.revision == i64::MAX as u64 || generation == i64::MAX as u64 {
            return Err(MemberError::AccessVersionExhausted.into());
        }
        let updated_at = at
            .format(&Rfc3339)
            .map_err(|_| MemberError::Stored("invalid access update timestamp"))?;
        let row = tx
            .query_one(
                "UPDATE users SET role=$2,active=$3,revision=revision+1,
            auth_generation=auth_generation+1,updated_at=$4::text::timestamptz WHERE id=$1
            RETURNING id,email,role,active,revision",
                &[
                    &id.as_uuid(),
                    &change.role().as_str(),
                    &change.active(),
                    &updated_at,
                ],
            )
            .map_err(port)?;
        let user = summary(&row)?;
        append_transaction(
            &mut tx,
            &email,
            "identity.user_access_changed",
            &format!(
                "user:{id}:revision:{}:from:{}:{}:to:{}:{}",
                user.revision,
                old.role.as_str(),
                old.active,
                user.role.as_str(),
                user.active
            ),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(user)
    }
}
