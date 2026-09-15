use application::cases::*;
use application::ApplicationError;
use domain::cases::{can_create_case, can_manage_members, CaseId};
use domain::identity::UserId;
use time::OffsetDateTime;

use super::{authorization, storage, PostgresCaseRepository};
use crate::audit_postgres::{append_transaction, begin_audited};
use storage::port;

pub(super) enum Change {
    Values(Box<CaseEditableValues>),
    Status(CaseAdministrativeStatus),
}
impl PostgresCaseRepository {
    pub(super) fn create(
        &self,
        actor: UserId,
        id: CaseId,
        values: CaseAdministrationValues,
        penal: bool,
        at: OffsetDateTime,
    ) -> Result<CaseAdministrationDetail, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::actor(&mut tx, actor, None)?;
        if !can_create_case(principal.role) {
            return Err(ApplicationError::PermissionDenied);
        }
        tx.execute(
            "INSERT INTO cases(id,title,reference,created_by) VALUES($1,$2,$3,$4)",
            &[
                &id.as_uuid(),
                &values.metadata().title(),
                &values.metadata().reference(),
                &actor.as_uuid(),
            ],
        )
        .map_err(port)?;
        tx.execute(
            "INSERT INTO case_memberships(case_id,user_id) VALUES($1,$2)",
            &[&id.as_uuid(), &actor.as_uuid()],
        )
        .map_err(port)?;
        let snapshot = storage::snapshot(
            id,
            CaseRevision::FIRST,
            values,
            &principal,
            at,
            self.hasher.as_ref(),
        );
        storage::insert(&mut tx, &snapshot)?;
        append_transaction(
            &mut tx,
            &principal.email,
            "case.created",
            &format!("case:{id}"),
            at,
        )?;
        append_transaction(
            &mut tx,
            &principal.email,
            "case.member_assigned",
            &format!("case:{id}:user:{actor}"),
            at,
        )?;
        let action = if penal {
            "case.penal_registered"
        } else {
            "case.administration_created"
        };
        append_transaction(
            &mut tx,
            &principal.email,
            action,
            &storage::resource(&snapshot),
            at,
        )?;
        if penal {
            tx.execute("INSERT INTO case_initial_stage_registrations(case_id,stage) VALUES($1,'investigation')", &[&id.as_uuid()]).map_err(port)?;
            append_transaction(
                &mut tx,
                &principal.email,
                "case.stage_initialized",
                &format!(
                    "case:{id}:stage:1:administration:1:sha256:{}",
                    snapshot.values_digest.to_hex()
                ),
                at,
            )?;
        }
        let detail = storage::detail(&mut tx, id, self.hasher.as_ref())?;
        tx.commit().map_err(port)?;
        Ok(detail)
    }
    pub(super) fn mutate(
        &self,
        actor: UserId,
        id: CaseId,
        expected: CaseRevisionExpectation,
        change: Change,
        at: OffsetDateTime,
    ) -> Result<CaseAdministrationDetail, ApplicationError> {
        let action = match &change {
            Change::Values(_) => CaseAdministrationAction::Replace,
            Change::Status(_) => CaseAdministrationAction::ChangeStatus,
        };
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::actor(&mut tx, actor, Some(action.permission()))?;
        authorization::require_scope(&mut tx, &principal, id)?;
        let current = storage::detail(&mut tx, id, self.hasher.as_ref())?;
        let values = current.administration.values();
        if matches!(&change, Change::Values(_))
            && values.status() == CaseAdministrativeStatus::Closed
        {
            return Err(ApplicationError::CaseClosed);
        }
        if current
            .administration
            .revision()
            .map_or(0, CaseRevision::get)
            != expected.get()
        {
            return Err(ApplicationError::CaseRevisionConflict);
        }
        let revision = current
            .administration
            .revision()
            .map_or(Some(CaseRevision::FIRST), CaseRevision::next)
            .ok_or(ApplicationError::CaseRevisionExhausted)?;
        let values = match change {
            Change::Values(editable) => {
                if values.profile().is_some() && editable.profile().is_none() {
                    return Err(ApplicationError::CaseProfileRequired);
                }
                CaseAdministrationValues::new(*editable, values.status())
            }
            Change::Status(status) => values.with_status(status),
        };
        let snapshot =
            storage::snapshot(id, revision, values, &principal, at, self.hasher.as_ref());
        storage::insert(&mut tx, &snapshot)?;
        append_transaction(
            &mut tx,
            &principal.email,
            action.audit_action(),
            &storage::resource(&snapshot),
            at,
        )?;
        let detail = storage::detail(&mut tx, id, self.hasher.as_ref())?;
        tx.commit().map_err(port)?;
        Ok(detail)
    }
    pub(super) fn member(
        &self,
        id: CaseId,
        user: UserId,
        actor: UserId,
        at: OffsetDateTime,
        add: bool,
    ) -> Result<(), ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::actor(&mut tx, actor, None)?;
        if !can_manage_members(principal.role) {
            return Err(ApplicationError::PermissionDenied);
        }
        authorization::require_scope(&mut tx, &principal, id)?;
        let changed=if add {
            tx.query_opt("SELECT id FROM users WHERE id=$1 AND active FOR SHARE", &[&user.as_uuid()]).map_err(port)?.ok_or(ApplicationError::UserNotFound)?;
            tx.execute("INSERT INTO case_memberships(case_id,user_id) VALUES($1,$2) ON CONFLICT(case_id,user_id) DO NOTHING", &[&id.as_uuid(),&user.as_uuid()])
        } else {tx.execute("DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2", &[&id.as_uuid(),&user.as_uuid()])}.map_err(port)?;
        if changed != 0 {
            append_transaction(
                &mut tx,
                &principal.email,
                if add {
                    "case.member_assigned"
                } else {
                    "case.member_removed"
                },
                &format!("case:{id}:user:{user}"),
                at,
            )?;
        }
        tx.commit().map_err(port)
    }
}
