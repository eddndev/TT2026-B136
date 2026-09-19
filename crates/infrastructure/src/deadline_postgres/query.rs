use super::{authorization, inconsistent, port, storage, PostgresDeadlineStore};
use application::{deadlines::*, ApplicationError};
use domain::{cases::CaseId, clock::OffsetDateTime, identity::UserId};
impl PostgresDeadlineStore {
    pub(super) fn list_page(
        &self,
        actor: UserId,
        case: CaseId,
        query: DeadlineQuery,
        at: OffsetDateTime,
    ) -> Result<DeadlinePage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
        let principal = authorization::actor(&mut tx, actor, case, false, self.hasher.as_ref())?;
        let after = query.after_id().map(|id| id.as_uuid());
        let status = query.status().status().map(|s| s.as_str());
        let limit = i64::from(query.limit()) + 1;
        let mut rows = tx.query("SELECT c.id,r.revision FROM case_deadlines c
            JOIN LATERAL (SELECT revision,status FROM case_deadline_revisions WHERE deadline_id=c.id ORDER BY revision DESC LIMIT 1) r ON TRUE
            WHERE c.case_id=$1 AND ($2::uuid IS NULL OR c.id>$2) AND ($3::text IS NULL OR r.status=$3)
            ORDER BY c.id LIMIT $4", &[&case.as_uuid(), &after, &status, &limit]).map_err(port)?;
        let has_more = rows.len() > query.limit() as usize;
        rows.truncate(query.limit() as usize);
        let mut deadlines = Vec::with_capacity(rows.len());
        for row in rows {
            let id = DeadlineId::from_uuid(row.try_get(0).map_err(inconsistent)?);
            let revision = number(row.try_get(1).map_err(inconsistent)?)?;
            let detail = storage::detail(&mut tx, case, id, Some(revision), self.hasher.as_ref())?;
            if query
                .status()
                .status()
                .is_some_and(|status| status != detail.status)
            {
                return Err(inconsistent(
                    "deadline status projection differs from captured state",
                ));
            }
            let current = self.current_projection(&mut tx, &detail)?;
            deadlines.push(DeadlineOverview::from(&current));
        }
        let next_after_id = if has_more {
            deadlines.last().map(|d| d.id)
        } else {
            None
        };
        crate::audit_postgres::append_transaction(
            &mut tx,
            &principal.email,
            "deadline.list_read",
            &format!("case:{case}:deadlines"),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(DeadlinePage {
            deadlines,
            has_more,
            next_after_id,
        })
    }
    pub(super) fn history_page(
        &self,
        actor: UserId,
        case: CaseId,
        id: DeadlineId,
        query: DeadlineHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<DeadlineHistoryPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = crate::audit_postgres::begin_audited(&mut client)?;
        let principal = authorization::actor(&mut tx, actor, case, false, self.hasher.as_ref())?;
        if !authorization::visible(&mut tx, case, id)? {
            return Err(DeadlineError::NotFound.into());
        }
        storage::detail(&mut tx, case, id, None, self.hasher.as_ref())?;
        let before = query.before_revision().map(|v| i64::from(v.get()));
        let limit = i64::from(query.limit()) + 1;
        let mut rows = tx
            .query(
                "SELECT revision FROM case_deadline_revisions WHERE deadline_id=$1 AND case_id=$2
            AND ($3::bigint IS NULL OR revision<$3) ORDER BY revision DESC LIMIT $4",
                &[&id.as_uuid(), &case.as_uuid(), &before, &limit],
            )
            .map_err(port)?;
        let has_more = rows.len() > query.limit() as usize;
        rows.truncate(query.limit() as usize);
        let mut revisions = Vec::with_capacity(rows.len());
        for row in rows {
            let revision = number(row.try_get(0).map_err(inconsistent)?)?;
            let detail = storage::detail(&mut tx, case, id, Some(revision), self.hasher.as_ref())?;
            revisions.push(DeadlineHistoryEntry::from_detail(
                self.hasher.as_ref(),
                &detail,
            )?);
        }
        let next_before_revision = if has_more {
            revisions.last().map(|r| r.revision)
        } else {
            None
        };
        crate::audit_postgres::append_transaction(
            &mut tx,
            &principal.email,
            "deadline.history_read",
            &format!("case:{case}:deadline:{id}:history"),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(DeadlineHistoryPage {
            revisions,
            has_more,
            next_before_revision,
        })
    }
}
fn number(value: i64) -> Result<DeadlineRevision, ApplicationError> {
    DeadlineRevision::new(u32::try_from(value).map_err(inconsistent)?).map_err(inconsistent)
}
