use application::participants::{
    ParticipantAction, ParticipantHistoryPage, ParticipantHistoryQuery, ParticipantId,
    ParticipantPage, ParticipantQuery,
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
        let rows=tx.query("SELECT p.case_id,r.* FROM case_participants p
            JOIN LATERAL (SELECT * FROM case_participant_revisions WHERE participant_id=p.id ORDER BY revision DESC LIMIT 1) r ON TRUE
            WHERE p.case_id=$1 AND ($2::uuid IS NULL OR p.id>$2)
                AND ($3::text IS NULL OR strpos(r.display_name COLLATE \"C\",$3)>0)
                AND ($4::text IS NULL OR r.procedural_role COLLATE \"C\"=$4)
                AND ($5::text IS NULL OR r.directory_status COLLATE \"C\"=$5)
            ORDER BY p.id LIMIT $6", &[&case.as_uuid(),&cursor,&query.name(),&query.procedural_role(),&status,&(i64::from(query.limit())+1)])
            .map_err(storage::port)?;
        let has_more = rows.len() > query.limit() as usize;
        let participants = rows
            .iter()
            .take(query.limit() as usize)
            .map(|r| storage::decode(r, self.hasher.as_ref()))
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
        let rows=tx.query("SELECT p.case_id,r.* FROM case_participant_revisions r JOIN case_participants p ON p.id=r.participant_id
            WHERE p.id=$1 AND p.case_id=$2 AND ($3::bigint IS NULL OR r.revision<$3) ORDER BY r.revision DESC LIMIT $4",
            &[&id.as_uuid(),&case.as_uuid(),&cursor,&(i64::from(query.limit())+1)]).map_err(storage::port)?;
        let has_more = rows.len() > query.limit() as usize;
        let revisions = rows
            .iter()
            .take(query.limit() as usize)
            .map(|r| storage::decode(r, self.hasher.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;
        let next_before_revision = has_more.then(|| revisions.last().unwrap().revision);
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
