use super::*;
use crate::ApplicationError;
use domain::cases::CaseId;

impl ResourceActivityService {
    pub(super) fn read_target_page(
        &self,
        token: &str,
        case: CaseId,
        target: ResourceActivityTargetId,
        query: ResourceActivityTargetQuery,
    ) -> Result<ResourceActivityTargetPage, ApplicationError> {
        let actor = self.actor(token, false)?;
        let started = self.clock.now();
        let page = self
            .store
            .list_for_target(actor.id, case, target, query, started)?;
        let returned = self.clock.now();
        super::reads::read_window(started, returned)?;
        super::validation::valid_time(page.checked_at)?;
        if page.checked_at < started || page.checked_at > returned {
            return Err(inconsistent(
                "target association observation differs from read window",
            ));
        }
        super::reads::page_shape(
            page.associations.len(),
            query.limit(),
            page.has_more,
            page.next_after_id,
            page.associations.last().map(|value| value.association.id),
        )?;
        let mut prior = query.after_id();
        let mut current = None;
        for view in &page.associations {
            let row = &view.association;
            self.view(view, case, row.resource_id, started, returned)?;
            if view.checked_at != page.checked_at
                || prior.is_some_and(|id| id.as_uuid() >= row.id.as_uuid())
                || query.status().is_some_and(|status| status != row.status)
                || !same_target(target, row.selection.target)
                || current.is_some_and(|value| value != &view.current_target)
            {
                return Err(inconsistent(
                    "target association scope, head, observation, order or filter differs",
                ));
            }
            prior = Some(row.id);
            current = Some(&view.current_target);
        }
        self.reauthenticate(token, &actor)?;
        Ok(page)
    }
}
fn same_target(wanted: ResourceActivityTargetId, actual: ResourceActivityTarget) -> bool {
    match (wanted, actual) {
        (ResourceActivityTargetId::Hearing(wanted), ResourceActivityTarget::Hearing { id, .. }) => {
            wanted == id
        }
        (
            ResourceActivityTargetId::Deadline(wanted),
            ResourceActivityTarget::Deadline { id, .. },
        ) => wanted == id,
        _ => false,
    }
}
