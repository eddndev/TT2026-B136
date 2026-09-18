use super::{authorization, inconsistent, port, PostgresDeadlineStore};
use application::{deadlines::*, ApplicationError};
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    identity::{Role, UserId},
};
use std::str::FromStr;

impl PostgresDeadlineStore {
    pub(super) fn responsible_page(
        &self,
        actor: UserId,
        case: CaseId,
        query: DeadlineResponsibleQuery,
        at: OffsetDateTime,
    ) -> Result<DeadlineResponsiblePage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
        let principal = authorization::actor(&mut tx, actor, case, false, self.hasher.as_ref())?;
        let after = query.after_id().map(|id| id.as_uuid());
        let limit = i64::from(query.limit()) + 1;
        let mut rows = tx
            .query(
                "SELECT u.id,u.email,u.role FROM users u
             WHERE u.active AND ($2::uuid IS NULL OR u.id>$2)
               AND (u.role='owner' OR (u.role IN ('litigator','paralegal') AND EXISTS (
                 SELECT 1 FROM case_memberships m WHERE m.case_id=$1 AND m.user_id=u.id)))
             ORDER BY u.id LIMIT $3",
                &[&case.as_uuid(), &after, &limit],
            )
            .map_err(port)?;
        let has_more = rows.len() > query.limit() as usize;
        rows.truncate(query.limit() as usize);
        let responsibles = rows
            .into_iter()
            .map(|row| {
                Ok(DeadlineResponsibleCandidate {
                    id: UserId::from_uuid(row.try_get(0).map_err(inconsistent)?),
                    email: row.try_get(1).map_err(inconsistent)?,
                    role: Role::from_str(row.try_get::<_, &str>(2).map_err(inconsistent)?)
                        .map_err(inconsistent)?,
                })
            })
            .collect::<Result<Vec<_>, ApplicationError>>()?;
        let next_after_id = if has_more {
            responsibles.last().map(|row| row.id)
        } else {
            None
        };
        let page = DeadlineResponsiblePage {
            case_id: case,
            responsibles,
            has_more,
            next_after_id,
        };
        page.validate(case, query)?;
        crate::audit_postgres::append_transaction(
            &mut tx,
            &principal.email,
            "deadline.responsibles_read",
            &format!("case:{case}:deadline-responsibles"),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(page)
    }
}
