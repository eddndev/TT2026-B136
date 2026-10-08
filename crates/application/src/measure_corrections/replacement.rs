use super::{record_view::RecordView, wire::invalid, *};
use crate::{
    identity::Principal,
    precautionary_measures::{
        resolve_measure_sources, MeasureDecisionRecordHistoryEvidence, MeasureSources,
    },
    typed_participants::SubjectSnapshot,
    ApplicationError,
};
use domain::{
    cases::CaseId,
    crypto::DocumentHasher,
    precautionary_hearings::MeasureRevision,
    precautionary_measures::{MeasureValues, MeasureValuesInput},
};

/// Marks an exact record and creates a separately rooted identity in one receipt.
/// Supplied history does not establish current head, durable absence or authority.
pub fn prepare_measure_administrative_replacement_with_decision_history(
    hasher: &dyn DocumentHasher,
    actor: &Principal,
    case_id: CaseId,
    command: MeasureAdministrativeCommand,
    material: MeasureAdministrativeReplacementMaterial,
    evidence: &MeasureDecisionRecordHistoryEvidence,
) -> Result<CheckedMeasureAdministrativeReview, ApplicationError> {
    if !matches!(
        command.action,
        MeasureAdministrativeAction::MarkEnteredInErrorAndReplace { .. }
    ) {
        return Err(invalid(
            "replacement entry point requires a replacement action",
        ));
    }
    super::preparation::prepare_with_view_with_subject(
        hasher,
        actor,
        case_id,
        command,
        material.context,
        evidence.into(),
        Some(material.subject),
    )
}

pub(super) fn result(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    command: &MeasureAdministrativeCommand,
    previous: RecordView<'_>,
    subject: Option<SubjectSnapshot>,
) -> Result<Option<MeasureAdministrativeResult>, ApplicationError> {
    let MeasureAdministrativeAction::MarkEnteredInErrorAndReplace {
        replacement_id,
        subject: selected,
    } = &command.action
    else {
        if subject.is_some() {
            return Err(invalid(
                "nonreplacement action has replacement subject material",
            ));
        }
        return Ok(None);
    };
    if *replacement_id == previous.reference().id() {
        return Err(invalid(
            "replacement must create a distinct measure identity",
        ));
    }
    let subject = subject.ok_or_else(|| invalid("replacement lacks its exact subject material"))?;
    let prior = previous.values();
    let values = MeasureValues::new(MeasureValuesInput {
        subject: *selected,
        kind: prior.kind(),
        conditions: prior.conditions().clone(),
        validity: prior.validity().clone(),
        supervision: prior.supervision().clone(),
    });
    let sources = MeasureSources {
        subject,
        supervisor: previous.sources().supervisor.clone(),
    };
    let projection = resolve_measure_sources(hasher, case_id, &values, &sources)?;
    Ok(Some(MeasureAdministrativeResult {
        id: *replacement_id,
        revision: MeasureRevision::new(1).map_err(|error| invalid(&error.to_string()))?,
        previous: command.target,
        record_root: MeasureRecordRoot::Administrative {
            operation_id: command.operation_id,
            measure_id: *replacement_id,
        },
        judicial_origin: previous.judicial_origin(),
        last_judicial: previous.judicial_reference(),
        last_action: previous.last_action(),
        validity: MeasureCaptureValidity::Valid,
        values,
        sources,
        projection,
    }))
}
