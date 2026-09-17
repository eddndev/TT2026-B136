use super::{FactSources, ProceduralFactError};
use crate::{typed_participants::ParticipantOverview, ApplicationError};
use domain::{crypto::ArchiveEntry, participants::ParticipantValues};

pub(super) fn validate(sources: &FactSources) -> Result<(), ApplicationError> {
    let resolved = &sources.resolved;
    let views = &sources.views;
    if resolved.participants.len() > 4
        || resolved.hearing_results.len() > 2
        || sources.direct_supports.len() > 2
        || resolved.participants.len() != views.participants.len()
        || resolved.hearing_results.len() != views.hearing_results.len()
    {
        return Err(inconsistent("source or view cardinality differs"));
    }
    match (&resolved.resolution, &views.resolution) {
        (None, None) => {}
        (Some(source), Some(view)) if source.reference == view.reference => {}
        _ => return Err(inconsistent("resolution source and view differ")),
    }
    if !strictly_ordered(&resolved.participants, |source| {
        (
            source.reference.id.as_uuid(),
            source.reference.revision.get(),
        )
    }) || !strictly_ordered(&resolved.hearing_results, |source| {
        let reference = source.reference;
        (
            reference.hearing_id.as_uuid(),
            reference.result_id.as_uuid(),
            reference.revision.get(),
            reference.agreement_id.map(|id| id.as_uuid()),
        )
    }) || !strictly_ordered(&sources.direct_supports, |source| {
        (
            source.reference.id.as_uuid(),
            source.reference.version.get(),
        )
    }) {
        return Err(inconsistent("source keys are not strictly increasing"));
    }
    for (source, view) in resolved.participants.iter().zip(&views.participants) {
        if source.case_id != view.case_id
            || source.reference.id != view.id
            || source.reference.revision != view.revision
            || source.status != view.directory_status
            || source.subject != view.subject
            || view.kind.is_some() != source.subject.is_some()
            || view
                .kind
                .is_some_and(|kind| view.procedural_role != kind.as_str())
        {
            return Err(inconsistent("participant source and view differ"));
        }
        validate_participant_text(view)?;
    }
    for (source, view) in resolved.hearing_results.iter().zip(&views.hearing_results) {
        if source.reference != view.reference
            || source.reference.agreement_id != view.agreement.as_ref().map(|value| value.id())
        {
            return Err(inconsistent("hearing result source and view differ"));
        }
    }
    for (pair, view_pair) in resolved
        .hearing_results
        .windows(2)
        .zip(views.hearing_results.windows(2))
    {
        let left = pair[0];
        let right = pair[1];
        let same_result = left.reference.hearing_id == right.reference.hearing_id
            && left.reference.result_id == right.reference.result_id
            && left.reference.revision == right.reference.revision;
        if same_result
            && (left.case_id != right.case_id
                || left.values_digest != right.values_digest
                || left.submission_digest != right.submission_digest
                || left.status != right.status
                || view_pair[0].occurrence != view_pair[1].occurrence
                || view_pair[0].event_time != view_pair[1].event_time
                || view_pair[0].summary != view_pair[1].summary)
        {
            return Err(inconsistent(
                "selections of one result contain conflicting snapshots",
            ));
        }
    }
    for support in &sources.direct_supports {
        ArchiveEntry::new(support.name.clone(), Vec::new())
            .map_err(|_| inconsistent("direct support name is not a safe archive entry"))?;
    }
    Ok(())
}

fn validate_participant_text(view: &ParticipantOverview) -> Result<(), ApplicationError> {
    let normalized = ParticipantValues::new(
        &view.display_name,
        &view.procedural_role,
        view.organization.as_deref(),
        None,
        view.directory_status,
    )
    .map_err(|_| inconsistent("participant view text exceeds its value policy"))?;
    if normalized.display_name() != view.display_name
        || normalized.procedural_role() != view.procedural_role
        || normalized.organization() != view.organization.as_deref()
    {
        return Err(inconsistent("participant view text is not normalized"));
    }
    Ok(())
}

fn strictly_ordered<T, K: Ord>(items: &[T], key: impl Fn(&T) -> K) -> bool {
    items.windows(2).all(|pair| key(&pair[0]) < key(&pair[1]))
}
fn inconsistent(message: &str) -> ApplicationError {
    ProceduralFactError::StoredInconsistent(message.into()).into()
}
