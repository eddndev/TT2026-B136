use super::{authorize, inconsistent, port, storage, write, PostgresResourceActivityStore};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{resource_activities::*, ApplicationError};
use domain::{cases::CaseId, clock::OffsetDateTime, crypto::DocumentHasher, identity::UserId};
use postgres::Transaction;

impl PostgresResourceActivityStore {
    pub(super) fn list_page(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        query: ResourceActivityQuery,
        at: OffsetDateTime,
    ) -> Result<ResourceActivityPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor, case, false)?;
        let at = self.observation_time(at)?;
        crate::procedural_resource_postgres::storage::detail(
            &mut tx,
            case,
            resource,
            None,
            self.hasher.as_ref(),
        )?;
        let rows = tx.query("SELECT p.id,r.revision FROM case_resource_activity_associations p
            CROSS JOIN LATERAL (SELECT revision,target_kind,status FROM case_resource_activity_association_revisions
                WHERE association_id=p.id AND case_id=p.case_id AND resource_id=p.resource_id ORDER BY revision DESC LIMIT 1) r
            WHERE p.case_id=$1 AND p.resource_id=$2 AND ($3::uuid IS NULL OR p.id>$3)
                AND ($4::text IS NULL OR r.target_kind=$4) AND ($5::text IS NULL OR r.status=$5)
            ORDER BY p.id LIMIT $6", &[&case.as_uuid(),&resource.as_uuid(),&query.after_id().map(|id|id.as_uuid()),
                &query.kind().map(|kind|kind.as_str()),&query.status().map(|status|status.as_str()),&(i64::from(query.limit())+1)]).map_err(port)?;
        let has_more = rows.len() > query.limit() as usize;
        let mut associations = Vec::new();
        for row in rows.iter().take(query.limit() as usize) {
            let detail = storage::detail(
                &mut tx,
                case,
                resource,
                ResourceActivityId::from_uuid(row.get("id")),
                Some(storage::revision(row.get("revision"))?),
                self.hasher.as_ref(),
            )?;
            associations.push(view(&mut tx, detail, self.hasher.as_ref(), at)?);
        }
        let next_after_id = if has_more {
            associations.last().map(|value| value.association.id)
        } else {
            None
        };
        append_transaction(
            &mut tx,
            &principal.email,
            "resource_activity.list",
            &format!("case:{case}:resource:{resource}"),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(ResourceActivityPage {
            associations,
            has_more,
            next_after_id,
        })
    }
    pub(super) fn get_detail(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        id: ResourceActivityId,
        revision: Option<ResourceActivityRevision>,
        at: OffsetDateTime,
    ) -> Result<ResourceActivityView, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor, case, false)?;
        let at = self.observation_time(at)?;
        let detail = storage::detail(&mut tx, case, resource, id, revision, self.hasher.as_ref())?;
        let result = view(&mut tx, detail, self.hasher.as_ref(), at)?;
        append_transaction(
            &mut tx,
            &principal.email,
            "resource_activity.read",
            &write::resource(&result.association),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(result)
    }
    pub(super) fn history_page(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        id: ResourceActivityId,
        query: ResourceActivityHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<ResourceActivityHistoryPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor, case, false)?;
        let at = self.observation_time(at)?;
        storage::detail(&mut tx, case, resource, id, None, self.hasher.as_ref())?;
        let rows = tx.query("SELECT revision FROM case_resource_activity_association_revisions
            WHERE association_id=$1 AND case_id=$2 AND resource_id=$3 AND ($4::bigint IS NULL OR revision<$4)
            ORDER BY revision DESC LIMIT $5", &[&id.as_uuid(),&case.as_uuid(),&resource.as_uuid(),
                &query.before_revision().map(|value|i64::from(value.get())),&(i64::from(query.limit())+1)]).map_err(port)?;
        let has_more = rows.len() > query.limit() as usize;
        let revisions = rows
            .iter()
            .take(query.limit() as usize)
            .map(|row| {
                storage::detail(
                    &mut tx,
                    case,
                    resource,
                    id,
                    Some(storage::revision(row.get("revision"))?),
                    self.hasher.as_ref(),
                )
            })
            .collect::<Result<Vec<_>, ApplicationError>>()?;
        let next_before_revision = if has_more {
            revisions.last().map(|value| value.revision)
        } else {
            None
        };
        append_transaction(
            &mut tx,
            &principal.email,
            "resource_activity.history",
            &format!("case:{case}:resource:{resource}:association:{id}"),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(ResourceActivityHistoryPage {
            revisions,
            has_more,
            next_before_revision,
        })
    }
    pub(super) fn observation_time(
        &self,
        started: OffsetDateTime,
    ) -> Result<OffsetDateTime, ApplicationError> {
        valid_time(started)?;
        let checked_at = self.clock.now().to_offset(time::UtcOffset::UTC);
        valid_time(checked_at)?;
        if checked_at < started {
            return Err(inconsistent(
                "association observation clock precedes request start",
            ));
        }
        Ok(checked_at)
    }
}
fn valid_time(at: OffsetDateTime) -> Result<(), ApplicationError> {
    if at.offset() != time::UtcOffset::UTC || !(1..=9999).contains(&at.year()) {
        return Err(inconsistent(
            "association observation time must be representable UTC",
        ));
    }
    Ok(())
}
fn view(
    tx: &mut Transaction<'_>,
    association: ResourceActivityDetail,
    hasher: &dyn DocumentHasher,
    at: OffsetDateTime,
) -> Result<ResourceActivityView, ApplicationError> {
    let target = match association.selection.target {
        ResourceActivityTarget::Hearing { id, .. } => ResourceActivityTargetId::Hearing(id),
        ResourceActivityTarget::Deadline { id, .. } => ResourceActivityTargetId::Deadline(id),
        ResourceActivityTarget::ResourceHearing { id, .. } => {
            ResourceActivityTargetId::ResourceHearing(id)
        }
    };
    let current = current_target(tx, association.case_id, target, hasher, at)?;
    with_current_target(association, current, at)
}
pub(super) fn current_target(
    tx: &mut Transaction<'_>,
    case: CaseId,
    target: ResourceActivityTargetId,
    hasher: &dyn DocumentHasher,
    at: OffsetDateTime,
) -> Result<ResourceActivityCurrentTarget, ApplicationError> {
    match target {
        ResourceActivityTargetId::ResourceHearing(id) => {
            let current =
                crate::resource_hearing_postgres::storage::detail(tx, case, id, None, hasher)?;
            if current.recorded_at > at {
                return Err(inconsistent("resource hearing head postdates observation"));
            }
            Ok(ResourceActivityCurrentTarget::ResourceHearing(Box::new(
                current,
            )))
        }
        ResourceActivityTargetId::Hearing(id) => {
            let current = crate::hearing_postgres::storage::detail(tx, case, id, None, hasher)?;
            if current.snapshot.recorded_at > at {
                return Err(inconsistent(
                    "hearing head postdates association observation",
                ));
            }
            Ok(ResourceActivityCurrentTarget::Hearing(Box::new(current)))
        }
        ResourceActivityTargetId::Deadline(id) => {
            let head = crate::deadline_postgres::storage::detail(tx, case, id, None, hasher)?;
            if head.recorded_at > at {
                return Err(inconsistent(
                    "deadline head postdates association observation",
                ));
            }
            let current = crate::deadline_postgres::current_in_transaction(tx, &head, hasher, at)?;
            Ok(ResourceActivityCurrentTarget::Deadline(Box::new(current)))
        }
    }
}
pub(super) fn with_current_target(
    association: ResourceActivityDetail,
    current: ResourceActivityCurrentTarget,
    at: OffsetDateTime,
) -> Result<ResourceActivityView, ApplicationError> {
    if association.recorded_at > at {
        return Err(inconsistent("association observation predates capture"));
    }
    let matches = match (association.selection.target, &current) {
        (
            ResourceActivityTarget::Hearing { id, revision, .. },
            ResourceActivityCurrentTarget::Hearing(value),
        ) => {
            value.snapshot.id == id
                && value.snapshot.case_id == association.case_id
                && value.snapshot.revision >= revision
        }
        (
            ResourceActivityTarget::Deadline { id, revision, .. },
            ResourceActivityCurrentTarget::Deadline(value),
        ) => {
            value.detail().id == id
                && value.detail().case_id == association.case_id
                && value.detail().revision >= revision
        }
        (
            ResourceActivityTarget::ResourceHearing { id, revision, .. },
            ResourceActivityCurrentTarget::ResourceHearing(value),
        ) => {
            value.review.command.hearing_id == id
                && value.review.case_id == association.case_id
                && value.review.command.resource.id == association.resource_id
                && value.revision >= revision
        }
        _ => false,
    };
    if !matches {
        return Err(inconsistent(
            "current target contradicts association capture",
        ));
    }
    Ok(ResourceActivityView {
        association,
        checked_at: at,
        current_target: current,
    })
}
