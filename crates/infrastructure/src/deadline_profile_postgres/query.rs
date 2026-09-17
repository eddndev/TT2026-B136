use super::{authorization, header, inconsistent, port, storage, PostgresDeadlineProfileStore};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{deadline_profiles::*, ApplicationError};
use domain::{clock::OffsetDateTime, identity::UserId};
impl PostgresDeadlineProfileStore {
    pub(super) fn list_page(
        &self,
        actor: UserId,
        collection: DeadlineProfileCollection,
        query: DeadlineProfileQuery,
        at: OffsetDateTime,
    ) -> Result<DeadlineProfilePage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal =
            authorization::actor(&mut tx, actor, collection, false, self.hasher.as_ref())?;
        let case = match collection {
            DeadlineProfileCollection::Global => None,
            DeadlineProfileCollection::ForCase(id) => Some(id.as_uuid()),
        };
        let after = query.after_id().map(|id| id.as_uuid());
        let status = query.status().status().map(|s| s.as_str());
        let limit = i64::from(query.limit()) + 1;
        let mut rows=tx.query("SELECT c.id,r.revision FROM deadline_profiles c
            JOIN LATERAL (SELECT revision,status FROM deadline_profile_revisions WHERE profile_id=c.id ORDER BY revision DESC LIMIT 1) r ON TRUE
            WHERE (c.case_id IS NULL OR c.case_id=$1) AND ($2::uuid IS NULL OR c.id>$2)
            AND ($3::text IS NULL OR r.status=$3) ORDER BY c.id LIMIT $4", &[&case,&after,&status,&limit]).map_err(port)?;
        let has_more = rows.len() > query.limit() as usize;
        rows.truncate(query.limit() as usize);
        let mut profiles = Vec::with_capacity(rows.len());
        for row in rows {
            let id = DeadlineProfileId::from_uuid(row.try_get(0).map_err(inconsistent)?);
            let revision = DeadlineProfileRevision::new(header::counter(
                row.try_get(1).map_err(inconsistent)?,
            )?)
            .map_err(inconsistent)?;
            let (entry, title) =
                storage::summary(&mut tx, id, Some(revision), self.hasher.as_ref())?;
            profiles.push(DeadlineProfileOverview {
                id: entry.id,
                revision: entry.revision,
                status: entry.status,
                algorithm: entry.algorithm,
                definition_digest: entry.definition_digest,
                title,
                scope: entry.scope,
            });
        }
        let next_after_id = if has_more {
            profiles.last().map(|p| p.id)
        } else {
            None
        };
        append_transaction(
            &mut tx,
            &principal.email,
            "deadline_profile.list_read",
            &collection_resource(collection),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(DeadlineProfilePage {
            profiles,
            has_more,
            next_after_id,
        })
    }
    pub(super) fn history_page(
        &self,
        actor: UserId,
        collection: DeadlineProfileCollection,
        id: DeadlineProfileId,
        query: DeadlineProfileHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<DeadlineProfileHistoryPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal =
            authorization::actor(&mut tx, actor, collection, false, self.hasher.as_ref())?;
        if !authorization::visible(&mut tx, collection, id, false)? {
            return Err(DeadlineProfileError::NotFound.into());
        }
        storage::summary(&mut tx, id, None, self.hasher.as_ref())?;
        let before = query.before_revision().map(|r| i64::from(r.get()));
        let limit = i64::from(query.limit()) + 1;
        let mut rows = tx
            .query(
                "SELECT revision FROM deadline_profile_revisions WHERE profile_id=$1
            AND ($2::bigint IS NULL OR revision<$2) ORDER BY revision DESC LIMIT $3",
                &[&id.as_uuid(), &before, &limit],
            )
            .map_err(port)?;
        let has_more = rows.len() > query.limit() as usize;
        rows.truncate(query.limit() as usize);
        let mut revisions = Vec::with_capacity(rows.len());
        for row in rows {
            let revision = DeadlineProfileRevision::new(header::counter(
                row.try_get(0).map_err(inconsistent)?,
            )?)
            .map_err(inconsistent)?;
            revisions.push(storage::summary(&mut tx, id, Some(revision), self.hasher.as_ref())?.0);
        }
        let next_before_revision = if has_more {
            revisions.last().map(|r| r.revision)
        } else {
            None
        };
        append_transaction(
            &mut tx,
            &principal.email,
            "deadline_profile.history_read",
            &format!("profile:{id}:history"),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(DeadlineProfileHistoryPage {
            revisions,
            has_more,
            next_before_revision,
        })
    }
}
fn collection_resource(collection: DeadlineProfileCollection) -> String {
    match collection {
        DeadlineProfileCollection::Global => "deadline_profiles:global".into(),
        DeadlineProfileCollection::ForCase(id) => format!("case:{id}:deadline_profiles"),
    }
}
