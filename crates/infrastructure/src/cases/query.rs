use application::cases::*;
use application::ApplicationError;
use domain::cases::CaseId;
use domain::identity::{Role, UserId};
use time::OffsetDateTime;

use super::{authorization, storage, values, PostgresCaseRepository};
use crate::audit_postgres::{append_transaction, begin_audited};
use storage::port;

impl PostgresCaseRepository {
    pub(super) fn read(
        &self,
        actor: UserId,
        id: CaseId,
        staff: bool,
        at: OffsetDateTime,
    ) -> Result<CaseAdministrationDetail, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::actor(
            &mut tx,
            actor,
            staff.then_some(CaseAdministrationAction::Read.permission()),
        )?;
        authorization::require_scope(&mut tx, &principal, id)?;
        let detail = storage::detail(&mut tx, id, self.hasher.as_ref())?;
        let action = if staff {
            CaseAdministrationAction::Read.audit_action()
        } else {
            "case.read"
        };
        let resource = if staff {
            storage::current_resource(&detail)
        } else {
            format!("case:{id}")
        };
        append_transaction(&mut tx, &principal.email, action, &resource, at)?;
        tx.commit().map_err(port)?;
        Ok(detail)
    }
    pub(super) fn basic_list(
        &self,
        actor: UserId,
        limit: u32,
        offset: u32,
        at: OffsetDateTime,
    ) -> Result<Vec<CaseRecord>, ApplicationError> {
        if !(1..=100).contains(&limit) {
            return Err(ApplicationError::InvalidInput(
                "case limit must be between 1 and 100".into(),
            ));
        }
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::actor(&mut tx, actor, None)?;
        let rows=tx.query("SELECT c.id FROM cases c WHERE $1 OR EXISTS(SELECT 1 FROM case_memberships m WHERE m.case_id=c.id AND m.user_id=$2) ORDER BY c.id LIMIT $3 OFFSET $4", &[&(principal.role==Role::Owner),&actor.as_uuid(),&i64::from(limit),&i64::from(offset)]).map_err(port)?;
        let mut cases = Vec::with_capacity(rows.len());
        for row in rows {
            cases.push(storage::basic(&storage::detail(
                &mut tx,
                CaseId::from_uuid(row.get(0)),
                self.hasher.as_ref(),
            )?));
        }
        append_transaction(&mut tx, &principal.email, "case.listed", "cases", at)?;
        tx.commit().map_err(port)?;
        Ok(cases)
    }
    pub(super) fn administrative_list(
        &self,
        actor: UserId,
        query: CaseAdministrationQuery,
        at: OffsetDateTime,
    ) -> Result<CaseAdministrationPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let action = CaseAdministrationAction::List;
        let principal = authorization::actor(&mut tx, actor, Some(action.permission()))?;
        let status = match query.status() {
            CaseStatusFilter::Active => Some("active"),
            CaseStatusFilter::Closed => Some("closed"),
            CaseStatusFilter::All => None,
        };
        let complete = match query.profile() {
            CaseProfileFilter::Complete => Some(true),
            CaseProfileFilter::Pending => Some(false),
            CaseProfileFilter::All => None,
        };
        let mut rows=tx.query("SELECT c.id FROM cases c LEFT JOIN LATERAL(SELECT * FROM case_administration_revisions r WHERE r.case_id=c.id ORDER BY revision DESC LIMIT 1) h ON TRUE
            WHERE ($1 OR EXISTS(SELECT 1 FROM case_memberships m WHERE m.case_id=c.id AND m.user_id=$2))
                AND ($3::uuid IS NULL OR c.id>$3) AND ($4::text IS NULL OR COALESCE(h.administrative_status,'active') COLLATE \"C\"=$4 COLLATE \"C\")
                AND ($5::boolean IS NULL OR (h.nuc IS NOT NULL)=$5)
                AND ($6::text IS NULL OR strpos(COALESCE(h.title,c.title) COLLATE \"C\",$6 COLLATE \"C\")>0)
                AND ($7::text IS NULL OR h.nuc COLLATE \"C\"=$7 COLLATE \"C\")
                AND ($8::text IS NULL OR h.judicial_case_number COLLATE \"C\"=$8 COLLATE \"C\")
            ORDER BY c.id LIMIT $9", &[&(principal.role==Role::Owner),&actor.as_uuid(),&query.after_id().map(CaseId::as_uuid),&status,&complete,&query.title(),&query.nuc(),&query.judicial_case_number(),&(i64::from(query.limit())+1)]).map_err(port)?;
        let has_more = rows.len() > query.limit() as usize;
        rows.truncate(query.limit() as usize);
        let mut cases = Vec::with_capacity(rows.len());
        for row in rows {
            cases.push(storage::overview(storage::detail(
                &mut tx,
                CaseId::from_uuid(row.get(0)),
                self.hasher.as_ref(),
            )?));
        }
        let next_after_id = has_more
            .then(|| cases.last().map(|c| c.origin.id))
            .flatten();
        append_transaction(
            &mut tx,
            &principal.email,
            action.audit_action(),
            "cases:administration",
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(CaseAdministrationPage {
            cases,
            has_more,
            next_after_id,
        })
    }
    pub(super) fn history(
        &self,
        actor: UserId,
        id: CaseId,
        query: CaseAdministrationHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<CaseAdministrationHistoryPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let action = CaseAdministrationAction::History;
        let principal = authorization::actor(&mut tx, actor, Some(action.permission()))?;
        authorization::require_scope(&mut tx, &principal, id)?;
        let mut rows=tx.query("SELECT * FROM case_administration_revisions WHERE case_id=$1 AND ($2::bigint IS NULL OR revision<$2) ORDER BY revision DESC LIMIT $3", &[&id.as_uuid(),&query.before_revision().map(|r|i64::from(r.get())),&(i64::from(query.limit())+1)]).map_err(port)?;
        let has_more = rows.len() > query.limit() as usize;
        rows.truncate(query.limit() as usize);
        let revisions = rows
            .iter()
            .map(|r| values::decode(r, self.hasher.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;
        let next_before_revision = has_more
            .then(|| revisions.last().map(|r| r.revision))
            .flatten();
        append_transaction(
            &mut tx,
            &principal.email,
            action.audit_action(),
            &format!("case:{id}:administration:history"),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(CaseAdministrationHistoryPage {
            revisions,
            has_more,
            next_before_revision,
        })
    }
}
