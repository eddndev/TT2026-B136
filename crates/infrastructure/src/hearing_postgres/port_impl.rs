use super::{authorization, port, preparation, sources, storage, PostgresHearingStore};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::documents::StageSupportReadLimits;
use application::{hearings::*, ApplicationError};
use domain::{cases::CaseId, clock::OffsetDateTime, identity::UserId};

impl HearingStore for PostgresHearingStore {
    fn context(
        &self,
        actor: UserId,
        case: CaseId,
        at: OffsetDateTime,
    ) -> Result<HearingCaseContext, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::authorize(&mut tx, actor, case, false)?;
        let context = sources::context(&mut tx, case, self.hasher.as_ref())?;
        append_transaction(
            &mut tx,
            &principal.email,
            "hearing.context_read",
            &format!("case:{case}:hearing-context"),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(context)
    }
    fn get(
        &self,
        actor: UserId,
        case: CaseId,
        id: HearingId,
        revision: Option<HearingRevision>,
        at: OffsetDateTime,
    ) -> Result<HearingDetail, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::authorize(&mut tx, actor, case, false)?;
        let detail = storage::detail(&mut tx, case, id, revision, self.hasher.as_ref())?;
        append_transaction(
            &mut tx,
            &principal.email,
            "hearing.read",
            &format!(
                "case:{case}:hearing:{id}:revision:{}:sha256:{}",
                detail.snapshot.revision.get(),
                detail.snapshot.values_digest.to_hex()
            ),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(detail)
    }
    fn history(
        &self,
        actor: UserId,
        case: CaseId,
        id: HearingId,
        query: HearingHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<HearingHistoryPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::authorize(&mut tx, actor, case, false)?;
        tx.query_opt(
            "SELECT id FROM case_hearings WHERE id=$1 AND case_id=$2",
            &[&id.as_uuid(), &case.as_uuid()],
        )
        .map_err(port)?
        .ok_or(HearingError::NotFound)?;
        let before = query.before_revision().map(|r| i64::from(r.get()));
        let limit = i64::from(query.limit()) + 1;
        let mut rows = tx
            .query(
                "SELECT revision FROM case_hearing_revisions WHERE hearing_id=$1 AND case_id=$2
            AND ($3::bigint IS NULL OR revision<$3) ORDER BY revision DESC LIMIT $4",
                &[&id.as_uuid(), &case.as_uuid(), &before, &limit],
            )
            .map_err(port)?;
        let has_more = rows.len() > query.limit() as usize;
        rows.truncate(query.limit() as usize);
        let revisions = rows
            .into_iter()
            .map(|row| {
                let revision = HearingRevision::new(super::decode::counter(row.get(0))?)
                    .map_err(super::inconsistent)?;
                storage::detail(&mut tx, case, id, Some(revision), self.hasher.as_ref())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let next_before_revision = if has_more {
            revisions.last().map(|d| d.snapshot.revision)
        } else {
            None
        };
        append_transaction(
            &mut tx,
            &principal.email,
            "hearing.history_read",
            &format!("case:{case}:hearing:{id}:history"),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(HearingHistoryPage {
            revisions,
            has_more,
            next_before_revision,
        })
    }
    fn prepare(
        &self,
        actor: UserId,
        case: CaseId,
        command: &HearingCommand,
        limits: &StageSupportReadLimits,
    ) -> Result<HearingPreparation, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        authorization::authorize(&mut tx, actor, case, true)?;
        let prepared = preparation::load(&mut tx, case, command, limits, self.hasher.as_ref())?;
        tx.rollback().map_err(port)?;
        Ok(prepared)
    }
    fn commit(
        &self,
        actor: UserId,
        case: CaseId,
        prepared: PreparedHearingChange,
    ) -> Result<HearingDetail, ApplicationError> {
        self.commit_change(actor, case, prepared)
    }
    fn list(
        &self,
        actor: UserId,
        case: CaseId,
        query: HearingQuery,
        at: OffsetDateTime,
    ) -> Result<HearingPage, ApplicationError> {
        self.list_page(actor, case, query, at)
    }
    fn agenda(
        &self,
        actor: UserId,
        query: HearingAgendaQuery,
        at: OffsetDateTime,
    ) -> Result<HearingAgendaPage, ApplicationError> {
        self.agenda_page(actor, query, at)
    }
}
