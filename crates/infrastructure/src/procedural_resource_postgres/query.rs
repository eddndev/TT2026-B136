use super::{authorization, port, storage, write, PostgresProceduralResourceStore};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{procedural_resources::*, ApplicationError};
use domain::{cases::CaseId, clock::OffsetDateTime, identity::UserId};

impl PostgresProceduralResourceStore {
    pub(super) fn list_page(
        &self,
        actor: UserId,
        case: CaseId,
        query: ResourceQuery,
        at: OffsetDateTime,
    ) -> Result<ResourcePage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::authorize(&mut tx, actor, case, false)?;
        let rows=tx.query("SELECT r.resource_id,r.revision FROM case_procedural_resources p
            CROSS JOIN LATERAL (SELECT resource_id,revision,kind,status FROM case_procedural_resource_revisions WHERE resource_id=p.id AND case_id=p.case_id ORDER BY revision DESC LIMIT 1) r
            WHERE p.case_id=$1 AND ($2::uuid IS NULL OR p.id>$2) AND ($3::text IS NULL OR r.kind=$3) AND ($4::text IS NULL OR r.status=$4)
            ORDER BY p.id LIMIT $5",&[&case.as_uuid(),&query.after_id().map(|r|r.as_uuid()),&query.kind().map(|k|k.as_str()),&query.status().map(|s|s.as_str()),&(i64::from(query.limit())+1)]).map_err(port)?;
        let has_more = rows.len() > query.limit() as usize;
        let resources = rows
            .iter()
            .take(query.limit() as usize)
            .map(|r| {
                storage::detail(
                    &mut tx,
                    case,
                    ResourceId::from_uuid(r.get(0)),
                    Some(storage::revision(r.get(1))?),
                    self.hasher.as_ref(),
                )
            })
            .collect::<Result<Vec<_>, ApplicationError>>()?;
        let next_after_id = if has_more {
            resources.last().map(|r| r.id)
        } else {
            None
        };
        append_transaction(
            &mut tx,
            &principal.email,
            "procedural_resource.list",
            &format!("case:{case}"),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(ResourcePage {
            resources,
            has_more,
            next_after_id,
        })
    }
    pub(super) fn get_detail(
        &self,
        actor: UserId,
        case: CaseId,
        id: ResourceId,
        revision: Option<ResourceRevision>,
        at: OffsetDateTime,
    ) -> Result<ResourceDetail, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::authorize(&mut tx, actor, case, false)?;
        let detail = storage::detail(&mut tx, case, id, revision, self.hasher.as_ref())?;
        append_transaction(
            &mut tx,
            &principal.email,
            "procedural_resource.read",
            &write::resource(&detail),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(detail)
    }
    pub(super) fn history_page(
        &self,
        actor: UserId,
        case: CaseId,
        id: ResourceId,
        query: ResourceHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<ResourceHistoryPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::authorize(&mut tx, actor, case, false)?;
        tx.query_opt(
            "SELECT id FROM case_procedural_resources WHERE id=$1 AND case_id=$2",
            &[&id.as_uuid(), &case.as_uuid()],
        )
        .map_err(port)?
        .ok_or(ProceduralResourceError::NotFound)?;
        let rows=tx.query("SELECT revision FROM case_procedural_resource_revisions WHERE resource_id=$1 AND case_id=$2 AND ($3::bigint IS NULL OR revision<$3) ORDER BY revision DESC LIMIT $4",
            &[&id.as_uuid(),&case.as_uuid(),&query.before_revision().map(|r|i64::from(r.get())),&(i64::from(query.limit())+1)]).map_err(port)?;
        let has_more = rows.len() > query.limit() as usize;
        let revisions = rows
            .iter()
            .take(query.limit() as usize)
            .map(|r| {
                storage::detail(
                    &mut tx,
                    case,
                    id,
                    Some(storage::revision(r.get(0))?),
                    self.hasher.as_ref(),
                )
            })
            .collect::<Result<Vec<_>, ApplicationError>>()?;
        let next_before_revision = if has_more {
            revisions.last().map(|r| r.revision)
        } else {
            None
        };
        append_transaction(
            &mut tx,
            &principal.email,
            "procedural_resource.history",
            &format!("case:{case}:resource:{id}"),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(ResourceHistoryPage {
            revisions,
            has_more,
            next_before_revision,
        })
    }
}
