use super::{receipt::inconsistent, service::support_error, *};
use crate::ApplicationError;
use domain::{cases::CaseId, crypto::Sha256Digest, identity::Permission};

impl ProceduralFactWorkflow for ProceduralFactService {
    fn list_resolutions(
        &self,
        token: &str,
        case_id: CaseId,
        query: ResolutionQuery,
    ) -> Result<ResolutionPage, ApplicationError> {
        let actor = self.actor(token, Permission::ReadProceduralFact)?;
        let page = self
            .store
            .list_resolutions(actor, case_id, query, self.clock.now())?;
        validate_page(
            page.resolutions.len(),
            query.limit(),
            page.has_more,
            page.next_after_id,
            page.resolutions.last().map(|row| row.root.id()),
        )?;
        let mut previous = query.after_id();
        for row in &page.resolutions {
            if row.root.case_id() != case_id
                || query
                    .status()
                    .status()
                    .is_some_and(|status| status != row.status)
                || previous.is_some_and(|id| row.root.id().as_uuid() <= id.as_uuid())
            {
                return Err(inconsistent(
                    "resolution list scope, status or order differs",
                ));
            }
            previous = Some(row.root.id());
        }
        Ok(page)
    }

    fn list_notifications(
        &self,
        token: &str,
        case_id: CaseId,
        resolution_id: ResolutionId,
        query: NotificationQuery,
    ) -> Result<NotificationPage, ApplicationError> {
        let actor = self.actor(token, Permission::ReadProceduralFact)?;
        let page = self.store.list_notifications(
            actor,
            case_id,
            resolution_id,
            query,
            self.clock.now(),
        )?;
        validate_page(
            page.notifications.len(),
            query.limit(),
            page.has_more,
            page.next_after_id,
            page.notifications.last().map(|row| row.root.id()),
        )?;
        let mut previous = query.after_id();
        for row in &page.notifications {
            if row.root.case_id() != case_id
                || row.root.resolution_id() != resolution_id
                || row.resolution.id != resolution_id
                || query
                    .status()
                    .status()
                    .is_some_and(|status| status != row.status)
                || previous.is_some_and(|id| row.root.id().as_uuid() <= id.as_uuid())
            {
                return Err(inconsistent(
                    "notification list scope, parent, status or order differs",
                ));
            }
            previous = Some(row.root.id());
        }
        Ok(page)
    }

    fn get(
        &self,
        token: &str,
        case_id: CaseId,
        target: FactTarget,
        revision: Option<FactRevision>,
    ) -> Result<FactDetail, ApplicationError> {
        let actor = self.actor(token, Permission::ReadProceduralFact)?;
        let detail = self
            .store
            .get(actor, case_id, target, revision, self.clock.now())?;
        if detail.snapshot.case_id() != case_id
            || detail.snapshot.target() != target
            || revision.is_some_and(|value| value != detail.snapshot.metadata().revision)
        {
            return Err(inconsistent("detail differs from requested exact fact"));
        }
        fact_receipt_matches(self.hasher.as_ref(), &detail)?;
        Ok(detail)
    }

    fn history(
        &self,
        token: &str,
        case_id: CaseId,
        target: FactTarget,
        query: FactHistoryQuery,
    ) -> Result<FactHistoryPage, ApplicationError> {
        let actor = self.actor(token, Permission::ReadProceduralFact)?;
        let page = self
            .store
            .history(actor, case_id, target, query, self.clock.now())?;
        validate_page(
            page.revisions.len(),
            query.limit(),
            page.has_more,
            page.next_before_revision,
            page.revisions.last().map(|row| row.metadata.revision),
        )?;
        let mut previous = query.before_revision();
        for row in &page.revisions {
            if row.case_id != case_id
                || row.target != target
                || previous.is_some_and(|revision| row.metadata.revision >= revision)
            {
                return Err(inconsistent(
                    "history scope, target or revision order differs",
                ));
            }
            fact_history_receipt_matches(self.hasher.as_ref(), row)?;
            previous = Some(row.metadata.revision);
        }
        Ok(page)
    }

    fn prepare(
        &self,
        token: &str,
        case_id: CaseId,
        command: ProceduralFactCommand,
    ) -> Result<FactDraft, ApplicationError> {
        self.prepare_command(token, case_id, command)
            .map_err(support_error)
    }

    fn submit(
        &self,
        token: &str,
        case_id: CaseId,
        command: ProceduralFactCommand,
        expected_submission_digest: Sha256Digest,
    ) -> Result<FactDetail, ApplicationError> {
        self.submit_command(token, case_id, command, expected_submission_digest)
            .map_err(support_error)
    }
}

fn validate_page<T: PartialEq>(
    length: usize,
    limit: u32,
    has_more: bool,
    next: Option<T>,
    last: Option<T>,
) -> Result<(), ApplicationError> {
    if length > limit as usize
        || (has_more && (length != limit as usize || last.is_none() || next != last))
        || (!has_more && next.is_some())
    {
        return Err(inconsistent(
            "page length and continuation cursor are inconsistent",
        ));
    }
    Ok(())
}
