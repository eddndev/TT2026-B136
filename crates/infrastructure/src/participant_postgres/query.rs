use application::participants::{
    ParticipantAction, ParticipantHistoryPage, ParticipantHistoryQuery, ParticipantId,
    ParticipantPage, ParticipantProfileFilter, ParticipantQuery,
};
use application::ApplicationError;
use domain::cases::CaseId;
use domain::identity::UserId;
use time::OffsetDateTime;

use super::{authorization::authorize, storage, PostgresParticipantStore};
use crate::audit_postgres::{append_transaction, begin_audited};

impl PostgresParticipantStore {
    pub(super) fn list_page(
        &self,
        actor: UserId,
        case: CaseId,
        query: ParticipantQuery,
        at: OffsetDateTime,
    ) -> Result<ParticipantPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor, case, ParticipantAction::List, false)?;
        let cursor = query.after_id().map(|id| id.as_uuid());
        let status = query.status().directory_status().map(|s| s.as_str());
        let kind = query.kind().map(|v| v.as_str());
        let profile = match query.profile() {
            ParticipantProfileFilter::All => 0i16,
            ParticipantProfileFilter::Manual => 1,
            ParticipantProfileFilter::Typed => 2,
        };
        let rows = tx
            .query(
                include_str!("overview.sql"),
                &[
                    &case.as_uuid(),
                    &cursor,
                    &query.name(),
                    &query.procedural_role(),
                    &status,
                    &kind,
                    &profile,
                    &(i64::from(query.limit()) + 1),
                ],
            )
            .map_err(storage::port)?;
        let has_more = rows.len() > query.limit() as usize;
        let participants = rows
            .iter()
            .take(query.limit() as usize)
            .map(super::overview::decode)
            .collect::<Result<Vec<_>, _>>()?;
        let next_after_id = has_more.then(|| participants.last().unwrap().id);
        append_transaction(
            &mut tx,
            &principal.email,
            ParticipantAction::List.audit_action(),
            &format!("case:{case}:participants"),
            at,
        )?;
        tx.commit().map_err(storage::port)?;
        Ok(ParticipantPage {
            participants,
            has_more,
            next_after_id,
        })
    }

    pub(super) fn history_page(
        &self,
        actor: UserId,
        case: CaseId,
        id: ParticipantId,
        query: ParticipantHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<ParticipantHistoryPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor, case, ParticipantAction::History, true)?;
        storage::current(&mut tx, case, id, self.hasher.as_ref())?;
        let cursor = query.before_revision().map(|r| i64::from(r.get()));
        let rows=tx.query("SELECT revision FROM (SELECT revision FROM case_participant_revisions WHERE participant_id=$1 UNION ALL SELECT revision FROM case_participant_typed_revisions WHERE participant_id=$1) r
            WHERE ($2::bigint IS NULL OR revision<$2) ORDER BY revision DESC LIMIT $3",
            &[&id.as_uuid(),&cursor,&(i64::from(query.limit())+1)]).map_err(storage::port)?;
        let has_more = rows.len() > query.limit() as usize;
        let mut revisions = Vec::new();
        for row in rows.iter().take(query.limit() as usize) {
            let revision = storage::revision(row.try_get(0).map_err(storage::port)?)?;
            revisions.push(storage::exact(
                &mut tx,
                case,
                id,
                revision,
                self.hasher.as_ref(),
            )?);
        }
        let next_before_revision = has_more.then(|| revisions.last().unwrap().revision_number());
        append_transaction(
            &mut tx,
            &principal.email,
            ParticipantAction::History.audit_action(),
            &format!("case:{case}:participant:{id}:history"),
            at,
        )?;
        tx.commit().map_err(storage::port)?;
        Ok(ParticipantHistoryPage {
            revisions,
            has_more,
            next_before_revision,
        })
    }
}
