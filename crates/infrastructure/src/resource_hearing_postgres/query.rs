use super::{inconsistent, port, replay, storage, PostgresResourceHearingStore};
use crate::{
    audit_postgres::{append_transaction, begin_audited},
    procedural_resource_postgres::authorize,
};
use application::{
    resource_activities::ResourceActivityError, resource_hearings::*, ApplicationError,
};
use domain::{
    cases::CaseId,
    identity::UserId,
    procedural_resources::ResourceId,
    resource_hearings::{ResourceHearingId, ResourceHearingRevision},
};

impl ResourceHearingReadStore for PostgresResourceHearingStore {
    fn list(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        query: ResourceHearingReadQuery,
    ) -> Result<ResourceHearingPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor, case, false)?;
        crate::procedural_resource_postgres::storage::detail(
            &mut tx,
            case,
            resource,
            None,
            self.hasher.as_ref(),
        )?;
        let rows = tx
            .query(
                "SELECT id FROM case_resource_hearings
            WHERE case_id=$1 AND resource_id=$2 AND ($3::uuid IS NULL OR id>$3)
            ORDER BY id LIMIT $4",
                &[
                    &case.as_uuid(),
                    &resource.as_uuid(),
                    &query.after_id().map(|id| id.as_uuid()),
                    &(i64::from(query.limit()) + 1),
                ],
            )
            .map_err(port)?;
        let has_more = rows.len() > usize::from(query.limit());
        let mut items = Vec::new();
        for row in rows.iter().take(usize::from(query.limit())) {
            let hearing = storage::detail(
                &mut tx,
                case,
                ResourceHearingId::from_uuid(row.get("id")),
                None,
                self.hasher.as_ref(),
            )?;
            items.push(
                replay::creation(&mut tx, hearing, self.hasher.as_ref()).map_err(creation_error)?,
            );
        }
        let next_after_id = if has_more {
            items.last().map(|v| v.origin.hearing_id)
        } else {
            None
        };
        append_transaction(
            &mut tx,
            &principal.email,
            "resource_hearing.list",
            &format!("case:{case}:resource:{resource}"),
            self.clock.now().to_offset(time::UtcOffset::UTC),
        )?;
        tx.commit().map_err(port)?;
        Ok(ResourceHearingPage {
            case_id: case,
            resource_id: resource,
            items,
            has_more,
            next_after_id,
        })
    }
    fn get(
        &self,
        actor: UserId,
        case: CaseId,
        resource: ResourceId,
        hearing: ResourceHearingId,
        revision: Option<ResourceHearingRevision>,
    ) -> Result<ResourceHearingCreation, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorize(&mut tx, actor, case, false)?;
        // Bind every path parent before resolving historical evidence.
        tx.query_opt(
            "SELECT id FROM case_resource_hearings WHERE id=$1 AND case_id=$2 AND resource_id=$3",
            &[&hearing.as_uuid(), &case.as_uuid(), &resource.as_uuid()],
        )
        .map_err(port)?
        .ok_or(ResourceActivityError::NotFound)?;
        let detail = storage::detail(&mut tx, case, hearing, revision, self.hasher.as_ref())?;
        let result =
            replay::creation(&mut tx, detail, self.hasher.as_ref()).map_err(creation_error)?;
        append_transaction(
            &mut tx,
            &principal.email,
            "resource_hearing.read",
            &replay::marker(&result),
            self.clock.now().to_offset(time::UtcOffset::UTC),
        )?;
        tx.commit().map_err(port)?;
        Ok(result)
    }
}

fn creation_error(error: ApplicationError) -> ApplicationError {
    match error {
        ApplicationError::ResourceActivity(
            ResourceActivityError::NotFound | ResourceActivityError::OperationConflict,
        ) => inconsistent("stored resource hearing has incomplete creation evidence"),
        other => other,
    }
}
