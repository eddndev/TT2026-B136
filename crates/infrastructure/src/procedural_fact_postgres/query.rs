use super::{
    authorization, decode, inconsistent, port, storage, target, PostgresProceduralFactStore,
};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{procedural_facts::*, ApplicationError};
use domain::{cases::CaseId, clock::OffsetDateTime, identity::UserId};
use uuid::Uuid;

pub(super) struct Listing {
    pub parent: Option<ResolutionId>,
    pub limit: u32,
    pub after: Option<Uuid>,
    pub status: FactStatusFilter,
}
impl PostgresProceduralFactStore {
    pub(super) fn list_page(
        &self,
        actor: UserId,
        case: CaseId,
        query: Listing,
        at: OffsetDateTime,
    ) -> Result<(Vec<FactDetail>, bool), ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::authorize(&mut tx, actor, case, false)?;
        let parent = query.parent.map(|r| r.as_uuid());
        let family = if parent.is_some() {
            "notification"
        } else {
            "resolution"
        };
        if let Some(id) = query.parent {
            storage::detail(
                &mut tx,
                case,
                FactTarget::Resolution(id),
                None,
                self.hasher.as_ref(),
            )
            .map_err(super::sources::missing)?;
        }
        let status = query.status.status().map(target::status);
        let limit = i64::from(query.limit) + 1;
        let mut rows=tx.query("SELECT id,revision FROM (SELECT DISTINCT ON(r.id) r.id,r.revision,r.status
            FROM case_procedural_fact_revisions r JOIN case_procedural_facts f USING(family,id,case_id)
            WHERE r.family=$1 AND r.case_id=$2 AND f.parent_resolution_id IS NOT DISTINCT FROM $3::uuid
            ORDER BY r.id,r.revision DESC) heads WHERE ($4::uuid IS NULL OR id>$4)
            AND ($5::text IS NULL OR status=$5) ORDER BY id LIMIT $6", &[&family,&case.as_uuid(),&parent,&query.after,&status,&limit]).map_err(port)?;
        let has_more = rows.len() > query.limit as usize;
        rows.truncate(query.limit as usize);
        let mut details = Vec::with_capacity(rows.len());
        for row in rows {
            let id = row.get(0);
            let revision = FactRevision::new(decode::counter(row.get(1))?).map_err(inconsistent)?;
            let wanted = match query.parent {
                None => FactTarget::Resolution(ResolutionId::from_uuid(id)),
                Some(resolution_id) => FactTarget::Notification {
                    id: NotificationId::from_uuid(id),
                    resolution_id,
                },
            };
            details.push(storage::detail(
                &mut tx,
                case,
                wanted,
                Some(revision),
                self.hasher.as_ref(),
            )?);
        }
        append_transaction(
            &mut tx,
            &principal.email,
            "procedural_fact.list_read",
            &format!(
                "case:{case}:{family}:parent:{}",
                parent
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "none".into())
            ),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok((details, has_more))
    }
    pub(super) fn history_page(
        &self,
        actor: UserId,
        case: CaseId,
        wanted: FactTarget,
        query: FactHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<FactHistoryPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::authorize(&mut tx, actor, case, false)?;
        storage::detail(&mut tx, case, wanted, None, self.hasher.as_ref())?;
        let (family, id, _) = target::parts(wanted);
        let before = query.before_revision().map(|v| i64::from(v.get()));
        let limit = i64::from(query.limit()) + 1;
        let mut rows=tx.query("SELECT revision FROM case_procedural_fact_revisions WHERE family=$1 AND id=$2 AND case_id=$3
            AND ($4::bigint IS NULL OR revision<$4) ORDER BY revision DESC LIMIT $5",&[&family,&id,&case.as_uuid(),&before,&limit]).map_err(port)?;
        let has_more = rows.len() > query.limit() as usize;
        rows.truncate(query.limit() as usize);
        let mut revisions = Vec::with_capacity(rows.len());
        for row in rows {
            let revision = FactRevision::new(decode::counter(row.get(0))?).map_err(inconsistent)?;
            let detail =
                storage::detail(&mut tx, case, wanted, Some(revision), self.hasher.as_ref())?;
            revisions.push(FactHistoryEntry::from(&detail.snapshot));
        }
        let next_before_revision = if has_more {
            revisions.last().map(|r| r.metadata.revision)
        } else {
            None
        };
        append_transaction(
            &mut tx,
            &principal.email,
            "procedural_fact.history_read",
            &format!("case:{case}:{family}:{id}:history"),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(FactHistoryPage {
            revisions,
            has_more,
            next_before_revision,
        })
    }
}
