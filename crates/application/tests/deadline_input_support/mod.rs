#![allow(dead_code)]
mod external;
#[path = "../procedural_fact_service_support/mod.rs"]
pub mod facts;
mod notices;
use application::{cases::CurrentCaseAdministration, deadline_inputs::*, procedural_facts::*};
use domain::{
    cases::CaseId,
    crypto::DocumentHasher,
    deadline_arithmetic::{ArithmeticRule, DayBasis, DayInclusion, FinalDayPolicy},
    deadline_triggers::*,
    identity::UserId,
    procedural_time::DeclaredProceduralTime,
};
#[allow(unused_imports)]
pub use external::{calendar, hearing, resign_calendar, resign_hearing};
#[allow(unused_imports)]
pub use notices::{notification, notification_input, resign_fact};
use std::{num::NonZeroU32, sync::Arc};
use uuid::Uuid;

pub fn hasher() -> Arc<dyn DocumentHasher + Send + Sync> {
    facts::hasher()
}
pub fn case_id() -> CaseId {
    CaseId::from_uuid(Uuid::from_u128(1))
}
pub fn actor() -> UserId {
    UserId::from_uuid(Uuid::from_u128(2))
}
pub fn date(value: &str) -> DeclaredProceduralTime {
    DeclaredProceduralTime::date(value.parse().unwrap(), None).unwrap()
}
pub fn resolution(revision: u32, withdrawn: bool, at: &str) -> FactDetail {
    let original = facts::values();
    let values = ResolutionValues::new(ResolutionValuesInput {
        class: original.class().clone(),
        subtype: None,
        issuer: original.issuer().clone(),
        issued_at: date(at),
        summary: original.summary().clone(),
        provenance: original.provenance().clone(),
    });
    let change = if withdrawn {
        FactChange::withdraw(
            FactRevision::new(revision - 1).unwrap(),
            facts::text("Withdrawn"),
        )
    } else if revision == 1 {
        FactChange::record(values.clone())
    } else {
        FactChange::correct(
            FactRevision::new(revision - 1).unwrap(),
            values.clone(),
            facts::text("Corrected"),
        )
    };
    let command = ProceduralFactCommand::Resolution(ResolutionCommand::new(
        FactOperationId::from_uuid(Uuid::from_u128(100 + u128::from(revision))),
        ResolutionId::from_uuid(Uuid::from_u128(10)),
        change,
    ));
    let detail = facts::detail(actor(), case_id(), &command, values, facts::empty());
    fact_receipt_matches(hasher().as_ref(), &detail).unwrap();
    detail
}
pub fn request(source: &DeadlineSourceDetail) -> DeadlineInputRequest {
    let (reference, field) = match source {
        DeadlineSourceDetail::Fact(detail) => match &detail.snapshot {
            ProceduralFactSnapshot::Resolution(s) => (
                TriggerSourceRef::Resolution(FactResolutionRef {
                    id: s.root.id(),
                    revision: s.metadata.revision,
                }),
                TriggerField::ResolutionIssuedAt,
            ),
            ProceduralFactSnapshot::Notification(s) => (
                TriggerSourceRef::Notification {
                    id: s.root.id(),
                    revision: s.metadata.revision,
                    resolution: s.values.resolution(),
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
    DeadlineInputRequest {
        trigger: TriggerSelection {
            case_id: case_id(),
            source: FactDeclaration::Known(reference),
            qualification: None,
        },
        requirement: TriggerRequirement::SourceField(field),
        rule: ArithmeticRule::Days {
            quantity: NonZeroU32::new(1).unwrap(),
            inclusion: DayInclusion::OnAnchor,
            basis: DayBasis::Natural,
            final_day: FinalDayPolicy::Preserve,
        },
        calendar: None,
    }
}
pub fn material(source: DeadlineSourceDetail) -> DeadlineInputMaterial {
    DeadlineInputMaterial {
        case_id: case_id(),
        administration: facts::preparation(case_id()).observed_administration,
        source_head: Some(source.clone()),
        source: Some(source),
        calendar: None,
        calendar_head: None,
    }
}
pub fn unknown() -> (DeadlineInputRequest, DeadlineInputMaterial) {
    let source = DeadlineSourceDetail::Fact(Box::new(resolution(1, false, "2026-01-06")));
    let mut request = request(&source);
    request.trigger.source = FactDeclaration::Unknown(facts::text("Source not identified"));
    let mut material = material(source);
    material.source = None;
    material.source_head = None;
    (request, material)
}
pub fn metadata_mut(detail: &mut FactDetail) -> &mut FactRevisionMetadata {
    match &mut detail.snapshot {
        ProceduralFactSnapshot::Resolution(s) => &mut s.metadata,
        ProceduralFactSnapshot::Notification(s) => &mut s.metadata,
    }
}
pub fn inconsistent(result: Result<TriggeredArithmetic, application::ApplicationError>) {
    assert!(
        matches!(
            result,
            Err(application::ApplicationError::DeadlineInput(
                DeadlineInputError::Inconsistent(_)
            ))
        ),
        "{result:?}"
    );
}
pub fn administration() -> CurrentCaseAdministration {
    facts::preparation(case_id()).observed_administration
}
