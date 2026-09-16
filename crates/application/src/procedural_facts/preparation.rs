use super::{receipt::inconsistent, *};
use crate::{
    case_stages::StageSupportSnapshot,
    documents::{StageDocumentFormat, StageFormatPolicy},
    ApplicationError,
};
use domain::{
    cases::CaseId,
    crypto::{DocumentHasher, DocumentVersionRef},
};

pub(super) fn validate_preparation(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    command: &ProceduralFactCommand,
    preparation: &mut FactPreparation,
) -> Result<ProceduralFactValues, ApplicationError> {
    if preparation.case_id != case_id {
        return Err(inconsistent("preparation belongs to another case"));
    }
    validate_fact_base(
        case_id,
        command,
        preparation.base.as_ref().map(|base| &base.snapshot),
    )?;
    if let Some(base) = &preparation.base {
        fact_receipt_matches(hasher, base)?;
    }
    validate_fact_administration(
        hasher,
        case_id,
        &preparation.observed_administration,
        preparation
            .base
            .as_ref()
            .map(|base| &base.snapshot.metadata().recorded_administration),
    )?;
    if command.action() == FactAction::Withdraw {
        let material = &preparation.source_material;
        if material.resolution.is_some()
            || !material.participants.is_empty()
            || !material.hearing_results.is_empty()
            || !preparation.records.is_empty()
        {
            return Err(inconsistent("withdrawal supplies replacement material"));
        }
        return preparation
            .base
            .as_ref()
            .map(|base| base.snapshot.values())
            .ok_or_else(|| inconsistent("withdrawal lacks its validated base"));
    }
    let values = match command {
        ProceduralFactCommand::Resolution(value) => ProceduralFactValues::Resolution(Box::new(
            value
                .change()
                .values()
                .ok_or_else(|| inconsistent("resolution command lacks values"))?
                .clone(),
        )),
        ProceduralFactCommand::Notification(value) => ProceduralFactValues::Notification(Box::new(
            value
                .change()
                .values()
                .ok_or_else(|| inconsistent("notification command lacks values"))?
                .clone(),
        )),
    };
    let selection = FactSourceSelection::from_values(&values);
    if preparation.records.len() != selection.direct_supports().len()
        || preparation.records.len() > 2
    {
        return Err(inconsistent(
            "direct document count differs from exact selection",
        ));
    }
    preparation
        .records
        .sort_by_key(|record| (record.id.as_uuid(), record.version.get()));
    for (record, reference) in preparation.records.iter().zip(selection.direct_supports()) {
        if (record.id, record.version) != (reference.reference().id, reference.reference().version)
        {
            return Err(inconsistent(
                "direct document identity differs from exact selection",
            ));
        }
        if record.digest != reference.digest() {
            return Err(ProceduralFactError::SupportDigestMismatch.into());
        }
    }
    Ok(values)
}
pub(super) fn resolve_sources(
    hasher: &dyn DocumentHasher,
    preparation: &FactPreparation,
    values: &ProceduralFactValues,
) -> Result<FactSources, ApplicationError> {
    let selection = FactSourceSelection::from_values(values);
    let material = &preparation.source_material;
    let resolution = resolve_fact_resolution(
        hasher,
        preparation.case_id,
        &selection,
        material.resolution.as_deref(),
    )?;
    let participants = resolve_fact_participants(
        hasher,
        preparation.case_id,
        &selection,
        &material.participants,
    )?;
    let hearings = resolve_fact_hearings(
        hasher,
        preparation.case_id,
        &selection,
        &material.hearing_results,
    )?;
    Ok(FactSources {
        resolved: FactResolvedSources {
            resolution: resolution.as_ref().map(|value| value.snapshot),
            participants: participants.iter().map(|value| value.snapshot).collect(),
            hearing_results: hearings.iter().map(|value| value.snapshot).collect(),
        },
        views: FactSourceViews {
            resolution: resolution.map(|value| value.view),
            participants: participants
                .into_iter()
                .map(|value| value.overview)
                .collect(),
            hearing_results: hearings.into_iter().map(|value| value.view).collect(),
        },
        direct_supports: vec![],
    })
}
pub(super) fn admitted_supports(
    preparation: &FactPreparation,
    formats: Vec<StageDocumentFormat>,
) -> Result<Vec<StageSupportSnapshot>, ApplicationError> {
    if formats.len() != preparation.records.len() {
        return Err(inconsistent("admitted support count differs"));
    }
    Ok(preparation
        .records
        .iter()
        .zip(formats)
        .map(|(record, format)| StageSupportSnapshot {
            reference: DocumentVersionRef {
                id: record.id,
                version: record.version,
            },
            digest: record.digest,
            name: record.name.clone(),
            format,
            policy: StageFormatPolicy::PdfDocxV1,
        })
        .collect())
}
/// A correction may replace a reference, but cannot silently rewrite one it retains.
pub(super) fn validate_retained(
    base: &FactSources,
    current: &FactSources,
) -> Result<(), ApplicationError> {
    if let (Some(previous), Some(source)) =
        (&base.resolved.resolution, &current.resolved.resolution)
    {
        if previous.reference == source.reference
            && (previous != source || base.views.resolution != current.views.resolution)
        {
            return Err(inconsistent("retained resolution projection changed"));
        }
    }
    for (source, view) in current
        .resolved
        .participants
        .iter()
        .zip(&current.views.participants)
    {
        if let Some(index) = base
            .resolved
            .participants
            .iter()
            .position(|previous| previous.reference == source.reference)
        {
            if base.resolved.participants[index] != *source
                || base.views.participants[index] != *view
            {
                return Err(inconsistent("retained participant projection changed"));
            }
        }
    }
    for (source, view) in current
        .resolved
        .hearing_results
        .iter()
        .zip(&current.views.hearing_results)
    {
        for (previous, previous_view) in base
            .resolved
            .hearing_results
            .iter()
            .zip(&base.views.hearing_results)
        {
            let same_result = previous.reference.hearing_id == source.reference.hearing_id
                && previous.reference.result_id == source.reference.result_id
                && previous.reference.revision == source.reference.revision;
            if same_result
                && (previous.case_id != source.case_id
                    || previous.values_digest != source.values_digest
                    || previous.submission_digest != source.submission_digest
                    || previous.status != source.status
                    || previous_view.occurrence != view.occurrence
                    || previous_view.event_time != view.event_time
                    || previous_view.summary != view.summary
                    || (previous.reference.agreement_id == source.reference.agreement_id
                        && previous_view.agreement != view.agreement))
            {
                return Err(inconsistent("retained hearing result snapshot changed"));
            }
        }
    }
    for source in &current.direct_supports {
        if base
            .direct_supports
            .iter()
            .any(|previous| previous.reference == source.reference && previous != source)
        {
            return Err(inconsistent("retained support projection changed"));
        }
    }
    Ok(())
}
