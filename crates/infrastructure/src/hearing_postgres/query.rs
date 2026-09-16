use super::{authorization, decode, port, storage, PostgresHearingStore};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{hearings::*, ApplicationError};
use domain::{
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::DocumentHasher,
    identity::{Role, UserId},
};
use postgres::{Row, Transaction};

fn overview(
    tx: &mut Transaction<'_>,
    row: &Row,
    hasher: &dyn DocumentHasher,
) -> Result<HearingOverview, ApplicationError> {
    let case = CaseId::from_uuid(row.get(0));
    let id = HearingId::from_uuid(row.get(1));
    let revision =
        HearingRevision::new(decode::counter(row.get(2))?).map_err(super::inconsistent)?;
    let detail = storage::detail(tx, case, id, Some(revision), hasher)?;
    let current = crate::cases::storage::detail(tx, case, hasher)?
        .administration
        .values();
    let value = detail.snapshot;
    Ok(HearingOverview {
        case_id: case,
        case_title: current.metadata().title().into(),
        case_reference: current.metadata().reference().into(),
        case_status: current.status(),
        id,
        revision,
        kind: value.values.kind(),
        scheduled_at: value.values.scheduled_at(),
        modality: value.values.modality(),
        status: value.status,
        participant_count: value.values.participants().len() as u8,
    })
}
impl PostgresHearingStore {
    pub(super) fn list_page(
        &self,
        actor: UserId,
        case: CaseId,
        query: HearingQuery,
        at: OffsetDateTime,
    ) -> Result<HearingPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::authorize(&mut tx, actor, case, false)?;
        let after = query.after_id().map(|id| id.as_uuid());
        let status = query.status().status().map(|s| s.as_str());
        let limit = i64::from(query.limit()) + 1;
        let mut rows = tx
            .query(
                "SELECT h.case_id,h.id,r.revision FROM case_hearings h
            CROSS JOIN LATERAL (SELECT revision,status FROM case_hearing_revisions r
                WHERE r.hearing_id=h.id ORDER BY revision DESC LIMIT 1) r
            WHERE h.case_id=$1 AND ($2::uuid IS NULL OR h.id>$2)
                AND ($3::text IS NULL OR r.status=$3) ORDER BY h.id LIMIT $4",
                &[&case.as_uuid(), &after, &status, &limit],
            )
            .map_err(port)?;
        let has_more = rows.len() > query.limit() as usize;
        rows.truncate(query.limit() as usize);
        let hearings = rows
            .iter()
            .map(|row| overview(&mut tx, row, self.hasher.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;
        let next_after_id = if has_more {
            hearings.last().map(|h| h.id)
        } else {
            None
        };
        append_transaction(
            &mut tx,
            &principal.email,
            "hearing.list_read",
            &format!("case:{case}:hearings"),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(HearingPage {
            hearings,
            has_more,
            next_after_id,
        })
    }
    pub(super) fn agenda_page(
        &self,
        actor: UserId,
        query: HearingAgendaQuery,
        at: OffsetDateTime,
    ) -> Result<HearingAgendaPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::actor(&mut tx, actor, false)?;
        let owner = principal.role == Role::Owner;
        let status = query.status().status().map(|s| s.as_str());
        let after_at = query.after().map(|c| c.at.utc().unix_timestamp());
        let after_id = query.after().map(|c| c.id.as_uuid());
        let limit = i64::from(query.limit()) + 1;
        let mut rows = tx.query("SELECT h.case_id,h.id,r.revision FROM case_hearings h
            CROSS JOIN LATERAL (SELECT revision,status,(values_view->'time'->>'seconds')::bigint AS instant
                FROM case_hearing_revisions r WHERE r.hearing_id=h.id ORDER BY revision DESC LIMIT 1) r
            WHERE ($1 OR EXISTS(SELECT 1 FROM case_memberships m WHERE m.case_id=h.case_id AND m.user_id=$2))
                AND r.instant >= $3 AND r.instant < $4 AND ($5::text IS NULL OR r.status=$5)
                AND ($6::bigint IS NULL OR (r.instant,h.id)>($6,$7::uuid))
            ORDER BY r.instant,h.id LIMIT $8",
            &[&owner,&actor.as_uuid(),&query.from().unix_timestamp(),&query.until().unix_timestamp(),&status,&after_at,&after_id,&limit]).map_err(port)?;
        let has_more = rows.len() > query.limit() as usize;
        rows.truncate(query.limit() as usize);
        let hearings = rows
            .iter()
            .map(|row| overview(&mut tx, row, self.hasher.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;
        let next_after = if has_more {
            hearings.last().map(|h| HearingAgendaCursor {
                at: HearingTime::new(h.scheduled_at.utc()).expect("validated hearing time"),
                id: h.id,
            })
        } else {
            None
        };
        append_transaction(
            &mut tx,
            &principal.email,
            "hearing.agenda_read",
            &format!(
                "hearings:agenda:{}:{}",
                query.from().unix_timestamp(),
                query.until().unix_timestamp()
            ),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(HearingAgendaPage {
            hearings,
            has_more,
            next_after,
        })
    }
}
