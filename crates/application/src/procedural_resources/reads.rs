use super::*;
use crate::ApplicationError;
use domain::{cases::CaseId, crypto::Sha256Digest};

impl ProceduralResourceWorkflow for ProceduralResourceService {
    fn list(
        &self,
        token: &str,
        case_id: CaseId,
        query: ResourceQuery,
    ) -> Result<ResourcePage, ApplicationError> {
        let actor = self.actor(token, false)?;
        let page = self
            .store
            .list(actor.id, case_id, query, self.clock.now())?;
        page_shape(
            page.resources.len(),
            query.limit(),
            page.has_more,
            page.next_after_id,
            page.resources.last().map(|v| v.id),
        )?;
        let mut previous = query.after_id();
        for detail in &page.resources {
            resource_receipt_matches(self.hasher.as_ref(), detail)?;
            if detail.case_id != case_id
                || previous.is_some_and(|id| id.as_uuid() >= detail.id.as_uuid())
                || query
                    .kind()
                    .is_some_and(|kind| kind != detail.values.kind())
                || query.status().is_some_and(|status| status != detail.status)
            {
                return Err(inconsistent("resource list scope, filter or order differs"));
            }
            previous = Some(detail.id);
        }
        self.reauthenticate(token, &actor)?;
        Ok(page)
    }
    fn get(
        &self,
        token: &str,
        case_id: CaseId,
        id: ResourceId,
        revision: Option<ResourceRevision>,
    ) -> Result<ResourceDetail, ApplicationError> {
        let actor = self.actor(token, false)?;
        let detail = self
            .store
            .get(actor.id, case_id, id, revision, self.clock.now())?;
        if detail.case_id != case_id
            || detail.id != id
            || revision.is_some_and(|r| r != detail.revision)
        {
            return Err(inconsistent(
                "resource detail differs from requested exact revision",
            ));
        }
        resource_receipt_matches(self.hasher.as_ref(), &detail)?;
        self.reauthenticate(token, &actor)?;
        Ok(detail)
    }
    fn history(
        &self,
        token: &str,
        case_id: CaseId,
        id: ResourceId,
        query: ResourceHistoryQuery,
    ) -> Result<ResourceHistoryPage, ApplicationError> {
        let actor = self.actor(token, false)?;
        let page = self
            .store
            .history(actor.id, case_id, id, query, self.clock.now())?;
        page_shape(
            page.revisions.len(),
            query.limit(),
            page.has_more,
            page.next_before_revision,
            page.revisions.last().map(|v| v.revision),
        )?;
        let mut previous = query.before_revision();
        for detail in &page.revisions {
            if detail.case_id != case_id
                || detail.id != id
                || previous.is_some_and(|r| r <= detail.revision)
            {
                return Err(inconsistent(
                    "resource history scope or descending order differs",
                ));
            }
            resource_receipt_matches(self.hasher.as_ref(), detail)?;
            previous = Some(detail.revision);
        }
        for pair in page.revisions.windows(2) {
            let new = &pair[0];
            let old = &pair[1];
            if new.revision.get() != old.revision.get() + 1
                || new.recorded_at < old.recorded_at
                || new.receipt.previous
                    != Some(ResourceRevisionRef {
                        revision: old.revision,
                        capture_digest: old.receipt.capture_digest,
                    })
            {
                return Err(inconsistent(
                    "resource history has a gap or changed previous receipt",
                ));
            }
        }
        self.reauthenticate(token, &actor)?;
        Ok(page)
    }
    fn prepare(
        &self,
        token: &str,
        case_id: CaseId,
        command: ResourceCommand,
    ) -> Result<ResourceDraft, ApplicationError> {
        self.prepare_command(token, case_id, command)
    }
    fn submit(
        &self,
        token: &str,
        case_id: CaseId,
        command: ResourceCommand,
        expected_submission_digest: Sha256Digest,
    ) -> Result<ResourceDetail, ApplicationError> {
        self.submit_command(token, case_id, command, expected_submission_digest)
    }
}
fn page_shape<T: PartialEq>(
    length: usize,
    limit: u32,
    has_more: bool,
    next: Option<T>,
    last: Option<T>,
) -> Result<(), ApplicationError> {
    if length > limit as usize
        || (has_more && (length != limit as usize || next.is_none() || next != last))
        || (!has_more && next.is_some())
    {
        return Err(inconsistent("resource page length or continuation differs"));
    }
    Ok(())
}
