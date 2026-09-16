use super::{
    authorization, inconsistent, port, preparation, query::Listing, storage, write,
    PostgresProceduralFactStore,
};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{documents::StageSupportReadLimits, procedural_facts::*, ApplicationError};
use domain::{cases::CaseId, clock::OffsetDateTime, identity::UserId};

impl ProceduralFactStore for PostgresProceduralFactStore {
    fn list_resolutions(
        &self,
        actor: UserId,
        case: CaseId,
        query: ResolutionQuery,
        at: OffsetDateTime,
    ) -> Result<ResolutionPage, ApplicationError> {
        let (details, has_more) = self.list_page(
            actor,
            case,
            Listing {
                parent: None,
                limit: query.limit(),
                after: query.after_id().map(|r| r.as_uuid()),
                status: query.status(),
            },
            at,
        )?;
        let resolutions = details
            .into_iter()
            .map(|d| {
                let ProceduralFactSnapshot::Resolution(v) = d.snapshot else {
                    return Err(inconsistent("resolution list family differs"));
                };
                Ok(ResolutionOverview {
                    root: v.root,
                    revision: v.metadata.revision,
                    status: v.metadata.status,
                    class: v.values.class().clone(),
                    issued_at: v.values.issued_at(),
                })
            })
            .collect::<Result<Vec<_>, ApplicationError>>()?;
        let next_after_id = if has_more {
            resolutions.last().map(|r| r.root.id())
        } else {
            None
        };
        Ok(ResolutionPage {
            resolutions,
            has_more,
            next_after_id,
        })
    }
    fn list_notifications(
        &self,
        actor: UserId,
        case: CaseId,
        parent: ResolutionId,
        query: NotificationQuery,
        at: OffsetDateTime,
    ) -> Result<NotificationPage, ApplicationError> {
        let (details, has_more) = self.list_page(
            actor,
            case,
            Listing {
                parent: Some(parent),
                limit: query.limit(),
                after: query.after_id().map(|r| r.as_uuid()),
                status: query.status(),
            },
            at,
        )?;
        let notifications = details
            .into_iter()
            .map(|d| {
                let ProceduralFactSnapshot::Notification(v) = d.snapshot else {
                    return Err(inconsistent("notification list family differs"));
                };
                Ok(NotificationOverview {
                    root: v.root,
                    revision: v.metadata.revision,
                    status: v.metadata.status,
                    resolution: v.values.resolution(),
                    outcome: v.values.outcome().clone(),
                    practiced_at: v.values.practiced_at(),
                })
            })
            .collect::<Result<Vec<_>, ApplicationError>>()?;
        let next_after_id = if has_more {
            notifications.last().map(|r| r.root.id())
        } else {
            None
        };
        Ok(NotificationPage {
            notifications,
            has_more,
            next_after_id,
        })
    }
    fn get(
        &self,
        actor: UserId,
        case: CaseId,
        target: FactTarget,
        revision: Option<FactRevision>,
        at: OffsetDateTime,
    ) -> Result<FactDetail, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let principal = authorization::authorize(&mut tx, actor, case, false)?;
        let detail = storage::detail(&mut tx, case, target, revision, self.hasher.as_ref())?;
        append_transaction(
            &mut tx,
            &principal.email,
            "procedural_fact.read",
            &write::resource(&detail.snapshot),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(detail)
    }
    fn history(
        &self,
        actor: UserId,
        case: CaseId,
        target: FactTarget,
        query: FactHistoryQuery,
        at: OffsetDateTime,
    ) -> Result<FactHistoryPage, ApplicationError> {
        self.history_page(actor, case, target, query, at)
    }
    fn prepare(
        &self,
        actor: UserId,
        case: CaseId,
        command: &ProceduralFactCommand,
        limits: &StageSupportReadLimits,
    ) -> Result<FactPreparation, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        authorization::authorize(&mut tx, actor, case, true)?;
        let preparation = preparation::load(&mut tx, case, command, limits, self.hasher.as_ref())?;
        tx.rollback().map_err(port)?;
        Ok(preparation)
    }
    fn commit(
        &self,
        actor: UserId,
        case: CaseId,
        prepared: PreparedFactChange,
    ) -> Result<FactDetail, ApplicationError> {
        self.commit_change(actor, case, prepared)
    }
}
