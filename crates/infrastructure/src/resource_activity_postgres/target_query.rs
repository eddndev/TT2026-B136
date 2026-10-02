use super::{authorize, port, query, storage, PostgresResourceActivityStore};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{resource_activities::*, ApplicationError};
use domain::{cases::CaseId, clock::OffsetDateTime, identity::UserId};

impl PostgresResourceActivityStore {
    pub(super) fn target_page(
        &self,
        actor: UserId,
        case: CaseId,
        target: ResourceActivityTargetId,
        filter: ResourceActivityTargetQuery,
        started: OffsetDateTime,
    ) -> Result<ResourceActivityTargetPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor, case, false)?;
        let checked_at = self.observation_time(started)?;
        let current =
            query::current_target(&mut tx, case, target, self.hasher.as_ref(), checked_at)?;
        let (kind, target_id) = match target {
            ResourceActivityTargetId::Hearing(id) => ("hearing", id.as_uuid()),
            ResourceActivityTargetId::Deadline(id) => ("deadline", id.as_uuid()),
        };
        let rows = tx
            .query(
                "SELECT p.id,p.resource_id,r.revision FROM case_resource_activity_associations p
            CROSS JOIN LATERAL (
                SELECT revision,target_kind,hearing_id,deadline_id,status
                FROM case_resource_activity_association_revisions
                WHERE association_id=p.id AND case_id=p.case_id AND resource_id=p.resource_id
                ORDER BY revision DESC LIMIT 1
            ) r
            WHERE p.case_id=$1 AND ($2::uuid IS NULL OR p.id>$2)
                AND r.target_kind=$3
                AND (($3='hearing' AND r.hearing_id=$4) OR ($3='deadline' AND r.deadline_id=$4))
                AND ($5::text IS NULL OR r.status=$5)
            ORDER BY p.id LIMIT $6",
                &[
                    &case.as_uuid(),
                    &filter.after_id().map(|id| id.as_uuid()),
                    &kind,
                    &target_id,
                    &filter.status().map(|value| value.as_str()),
                    &(i64::from(filter.limit()) + 1),
                ],
            )
            .map_err(port)?;
        let has_more = rows.len() > filter.limit() as usize;
        let mut associations = Vec::with_capacity(rows.len().min(filter.limit() as usize));
        for row in rows.iter().take(filter.limit() as usize) {
            let detail = storage::detail(
                &mut tx,
                case,
                ResourceId::from_uuid(row.get("resource_id")),
                ResourceActivityId::from_uuid(row.get("id")),
                Some(storage::revision(row.get("revision"))?),
                self.hasher.as_ref(),
            )?;
            associations.push(query::with_current_target(
                detail,
                current.clone(),
                checked_at,
            )?);
        }
        let next_after_id = if has_more {
            associations.last().map(|view| view.association.id)
        } else {
            None
        };
        append_transaction(
            &mut tx,
            &principal.email,
            "resource_activity.target_list",
            &format!("case:{case}:{kind}:{target_id}"),
            checked_at,
        )?;
        tx.commit().map_err(port)?;
        Ok(ResourceActivityTargetPage {
            checked_at,
            associations,
            has_more,
            next_after_id,
        })
    }
}
