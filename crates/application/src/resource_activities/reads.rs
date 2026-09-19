use super::*;
use crate::{
    deadline_tracking::DeadlineReviewState,
    deadlines::{deadline_receipt_matches, DeadlineStatus},
    hearings::hearing_receipt_matches,
    ApplicationError,
};
use domain::{cases::CaseId, clock::OffsetDateTime, crypto::Sha256Digest};

impl ResourceActivityWorkflow for ResourceActivityService {
    fn list(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        query: ResourceActivityQuery,
    ) -> Result<ResourceActivityPage, ApplicationError> {
        let actor = self.actor(token, false)?;
        let at = self.clock.now();
        let page = self.store.list(actor.id, case, resource, query, at)?;
        page_shape(
            page.associations.len(),
            query.limit(),
            page.has_more,
            page.next_after_id,
            page.associations.last().map(|v| v.association.id),
        )?;
        let mut prior = query.after_id();
        for view in &page.associations {
            self.view(view, case, resource, at)?;
            let row = &view.association;
            if prior.is_some_and(|id| id.as_uuid() >= row.id.as_uuid())
                || query
                    .kind()
                    .is_some_and(|kind| kind != row.selection.target.kind())
                || query.status().is_some_and(|status| status != row.status)
            {
                return Err(inconsistent(
                    "association list order, scope or filter differs",
                ));
            }
            prior = Some(row.id);
        }
        self.reauthenticate(token, &actor)?;
        Ok(page)
    }
    fn get(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        id: ResourceActivityId,
        revision: Option<ResourceActivityRevision>,
    ) -> Result<ResourceActivityView, ApplicationError> {
        let actor = self.actor(token, false)?;
        let at = self.clock.now();
        let view = self.store.get(actor.id, case, resource, id, revision, at)?;
        self.view(&view, case, resource, at)?;
        if view.association.id != id || revision.is_some_and(|r| r != view.association.revision) {
            return Err(inconsistent(
                "association detail identity or exact revision differs",
            ));
        }
        self.reauthenticate(token, &actor)?;
        Ok(view)
    }
    fn history(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        id: ResourceActivityId,
        query: ResourceActivityHistoryQuery,
    ) -> Result<ResourceActivityHistoryPage, ApplicationError> {
        let actor = self.actor(token, false)?;
        let page = self
            .store
            .history(actor.id, case, resource, id, query, self.clock.now())?;
        page_shape(
            page.revisions.len(),
            query.limit(),
            page.has_more,
            page.next_before_revision,
            page.revisions.last().map(|v| v.revision),
        )?;
        let mut prior = query.before_revision();
        for row in &page.revisions {
            resource_activity_receipt_matches(self.hasher.as_ref(), row)?;
            if row.case_id != case
                || row.resource_id != resource
                || row.id != id
                || prior.is_some_and(|r| r <= row.revision)
            {
                return Err(inconsistent("association history scope or order differs"));
            }
            prior = Some(row.revision);
        }
        for pair in page.revisions.windows(2) {
            let (new, old) = (&pair[0], &pair[1]);
            if new.revision.get() != old.revision.get() + 1
                || new.recorded_at < old.recorded_at
                || new.receipt.previous
                    != Some(ResourceActivityRevisionRef {
                        revision: old.revision,
                        capture_digest: old.receipt.capture_digest,
                    })
                || new.selection != old.selection
                || new.sources != old.sources
            {
                return Err(inconsistent(
                    "association history changed its predecessor or exact endpoints",
                ));
            }
        }
        self.reauthenticate(token, &actor)?;
        Ok(page)
    }
    fn prepare(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        command: ResourceActivityCommand,
    ) -> Result<ResourceActivityDraft, ApplicationError> {
        self.prepare_command(token, case, resource, command)
    }
    fn submit(
        &self,
        token: &str,
        case: CaseId,
        resource: ResourceId,
        command: ResourceActivityCommand,
        expected_submission_digest: Sha256Digest,
    ) -> Result<ResourceActivityDetail, ApplicationError> {
        self.submit_command(token, case, resource, command, expected_submission_digest)
    }
}
impl ResourceActivityService {
    fn view(
        &self,
        view: &ResourceActivityView,
        case: CaseId,
        resource: ResourceId,
        at: OffsetDateTime,
    ) -> Result<(), ApplicationError> {
        let row = &view.association;
        resource_activity_receipt_matches(self.hasher.as_ref(), row)?;
        super::validation::valid_time(view.checked_at)?;
        if row.case_id != case
            || row.resource_id != resource
            || view.checked_at != at
            || row.recorded_at > at
        {
            return Err(inconsistent(
                "association current view scope or observation time differs",
            ));
        }
        match (
            row.selection.target,
            &row.sources.target,
            &view.current_target,
        ) {
            (
                ResourceActivityTarget::Hearing { id, revision, .. },
                ResourceActivityTargetDetail::Hearing(captured),
                ResourceActivityCurrentTarget::Hearing(current),
            ) => {
                hearing_receipt_matches(self.hasher.as_ref(), current)?;
                if current.snapshot.case_id != case
                    || current.snapshot.id != id
                    || current.snapshot.revision < revision
                    || current.snapshot.recorded_at > at
                    || (current.snapshot.revision == revision && current != captured)
                {
                    return Err(inconsistent(
                        "current hearing head contradicts its linked capture",
                    ));
                }
            }
            (
                ResourceActivityTarget::Deadline { id, revision, .. },
                ResourceActivityTargetDetail::Deadline(captured),
                ResourceActivityCurrentTarget::Deadline(current),
            ) => {
                let detail = current.detail();
                let operational = current.operational();
                deadline_receipt_matches(self.hasher.as_ref(), detail)?;
                let unchecked = detail.status == DeadlineStatus::Retired
                    || detail.review_state() == DeadlineReviewState::LegacyUndeclared;
                if detail.case_id != case
                    || detail.id != id
                    || detail.revision < revision
                    || detail.recorded_at > at
                    || (detail.revision == revision && detail != captured.as_ref())
                    || !operational.matches_capture(detail)
                    || match operational.checked_at() {
                        Some(checked) => checked != at,
                        None => !unchecked,
                    }
                {
                    return Err(inconsistent(
                        "current deadline head or observation contradicts its linked capture",
                    ));
                }
            }
            _ => {
                return Err(inconsistent(
                    "current activity family differs from linked target",
                ))
            }
        }
        Ok(())
    }
}
fn page_shape<T: PartialEq>(
    length: usize,
    limit: u32,
    more: bool,
    next: Option<T>,
    last: Option<T>,
) -> Result<(), ApplicationError> {
    if length > limit as usize
        || (more && (length != limit as usize || next.is_none() || next != last))
        || (!more && next.is_some())
    {
        return Err(inconsistent(
            "association page length or continuation differs",
        ));
    }
    Ok(())
}
