#![allow(dead_code)]

mod context;

use crate::deadline_input_support::{actor, case_id, hasher};
use application::{
    cases::{
        case_administration_digest, CaseActorSnapshot, CaseAdministrationSnapshot,
        CurrentCaseAdministration,
    },
    deadline_inputs::{DeadlineInputMaterial, DeadlineSourceDetail},
    procedural_facts::{FactHearingRef, FactResolutionRef, ProceduralFactSnapshot},
    ApplicationError,
};
use domain::{
    case_administration::{CaseAdministrationValues, CaseAdministrativeStatus, CaseRevision},
    cases::{CaseId, CaseMetadata},
    deadline_triggers::{TriggerField, TriggerRequirement, TriggerSelection, TriggerSourceRef},
    procedural_facts::{FactDeclaration, FactText},
};

pub fn selection(source: &DeadlineSourceDetail) -> (TriggerRequirement, TriggerSelection) {
    let (reference, field) = match source {
        DeadlineSourceDetail::Fact(detail) => match &detail.snapshot {
            ProceduralFactSnapshot::Resolution(snapshot) => (
                TriggerSourceRef::Resolution(FactResolutionRef {
                    id: snapshot.root.id(),
                    revision: snapshot.metadata.revision,
                }),
                TriggerField::ResolutionIssuedAt,
            ),
            ProceduralFactSnapshot::Notification(snapshot) => (
                TriggerSourceRef::Notification {
                    id: snapshot.root.id(),
                    revision: snapshot.metadata.revision,
                    resolution: snapshot.values.resolution(),
                },
                TriggerField::NotificationPracticedAt,
            ),
        },
        DeadlineSourceDetail::HearingResult(detail) => (
            TriggerSourceRef::HearingResult(FactHearingRef {
                hearing_id: detail.snapshot.hearing_id,
                result_id: detail.snapshot.id,
                revision: detail.snapshot.revision,
                agreement_id: None,
            }),
            TriggerField::HearingSessionEventTime,
        ),
    };
    (
        TriggerRequirement::SourceField(field),
        TriggerSelection {
            case_id: case_id(),
            source: FactDeclaration::Known(reference),
            qualification: None,
        },
    )
}

pub fn unknown() -> (TriggerSelection, DeadlineInputMaterial) {
    (
        TriggerSelection {
            case_id: case_id(),
            source: FactDeclaration::Unknown(FactText::new("Source not identified").unwrap()),
            qualification: None,
        },
        DeadlineInputMaterial {
            case_id: case_id(),
            administration: crate::deadline_input_support::administration(),
            source: None,
            source_head: None,
            calendar: None,
            calendar_head: None,
        },
    )
}

pub fn recorded(case: CaseId, status: CaseAdministrativeStatus) -> CurrentCaseAdministration {
    let values = CaseAdministrationValues::basic(CaseMetadata::new("Case", "REF-1").unwrap())
        .with_status(status);
    CurrentCaseAdministration::Recorded(Box::new(CaseAdministrationSnapshot {
        case_id: case,
        revision: CaseRevision::FIRST,
        values_digest: case_administration_digest(hasher().as_ref(), &values),
        values,
        changed_at: crate::case_support::instant(),
        changed_by: CaseActorSnapshot {
            id: actor(),
            email: "owner@example.test".into(),
        },
    }))
}

pub fn inconsistent<T: std::fmt::Debug>(result: Result<T, ApplicationError>) {
    assert!(
        matches!(
            result,
            Err(ApplicationError::DeadlineInput(
                application::deadline_inputs::DeadlineInputError::Inconsistent(_)
            ))
        ),
        "{result:?}"
    );
}
