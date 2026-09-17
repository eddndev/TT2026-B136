#![allow(dead_code)]

mod sources;
pub use sources::*;

pub use crate::case_administration_support::Fixture;
use crate::case_stage_database_support::FixedClock;
use application::{deadline_inputs::*, procedural_facts::*};
use domain::{
    cases::CaseId,
    deadline_arithmetic::{ArithmeticRule, DayBasis, DayInclusion, FinalDayPolicy},
    deadline_triggers::*,
    procedural_time::DeclaredProceduralTime,
};
use infrastructure::{PostgresDeadlineInputStore, RingSha256Hasher};
use std::{num::NonZeroU32, sync::Arc};

pub fn store(db: &Fixture) -> PostgresDeadlineInputStore {
    PostgresDeadlineInputStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
    .unwrap()
}

pub fn request(case_id: CaseId, source: TriggerSourceRef) -> DeadlineInputRequest {
    let field = match source {
        TriggerSourceRef::Resolution(_) => TriggerField::ResolutionIssuedAt,
        TriggerSourceRef::Notification { .. } => TriggerField::NotificationPracticedAt,
        TriggerSourceRef::HearingResult(_) => TriggerField::HearingSessionEventTime,
    };
    DeadlineInputRequest {
        trigger: TriggerSelection {
            case_id,
            source: FactDeclaration::Known(source),
            qualification: None,
        },
        requirement: TriggerRequirement::SourceField(field),
        rule: ArithmeticRule::Days {
            quantity: NonZeroU32::new(1).unwrap(),
            inclusion: DayInclusion::AfterAnchor,
            basis: DayBasis::Natural,
            final_day: FinalDayPolicy::Preserve,
        },
        calendar: None,
    }
}

pub fn unknown(case_id: CaseId) -> DeadlineInputRequest {
    let mut request = request(
        case_id,
        TriggerSourceRef::Resolution(FactResolutionRef {
            id: ResolutionId::new(),
            revision: FactRevision::initial(),
        }),
    );
    request.trigger.source = FactDeclaration::Unknown(FactText::new("Source not stated").unwrap());
    request
}

pub fn fact_request(detail: &FactDetail) -> DeadlineInputRequest {
    let metadata = detail.snapshot.metadata();
    let source = match &detail.snapshot {
        ProceduralFactSnapshot::Resolution(snapshot) => {
            TriggerSourceRef::Resolution(FactResolutionRef {
                id: snapshot.root.id(),
                revision: metadata.revision,
            })
        }
        ProceduralFactSnapshot::Notification(snapshot) => TriggerSourceRef::Notification {
            id: snapshot.root.id(),
            revision: metadata.revision,
            resolution: snapshot.values.resolution(),
        },
    };
    request(detail.snapshot.case_id(), source)
}

pub fn dated_resolution(date: &str) -> ProceduralFactCommand {
    let values = crate::procedural_fact_backend_support::values("Dated resolution");
    ProceduralFactCommand::Resolution(ResolutionCommand::new(
        FactOperationId::new(),
        ResolutionId::new(),
        FactChange::record(ResolutionValues::new(ResolutionValuesInput {
            class: values.class().clone(),
            subtype: None,
            issuer: values.issuer().clone(),
            issued_at: DeclaredProceduralTime::date(date.parse().unwrap(), None).unwrap(),
            summary: values.summary().clone(),
            provenance: values.provenance().clone(),
        })),
    ))
}

pub fn audit(db: &mut Fixture) -> serde_json::Value {
    db.admin.query_one(
        "SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY sequence),'[]'::jsonb) FROM audit_events a",
        &[],
    ).unwrap().get(0)
}

pub fn assert_one_read(db: &mut Fixture, before: &serde_json::Value) {
    let after = audit(db);
    let old = before.as_array().unwrap();
    let new = after.as_array().unwrap();
    assert_eq!(&new[..old.len()], old);
    assert_eq!(new.len(), old.len() + 1);
    assert_eq!(new.last().unwrap()["action"], "deadline.inputs_read");
    assert_eq!(
        new.last().unwrap()["resource"],
        format!("case:{}:deadline_inputs", db.case)
    );
}

pub fn assert_candidate(
    request: &DeadlineInputRequest,
    material: &DeadlineInputMaterial,
    date: &str,
) {
    let calculation = check_deadline_inputs(&RingSha256Hasher, request, material).unwrap();
    assert_eq!(
        calculation.arithmetic().unwrap().outcome(),
        &domain::deadline_arithmetic::ArithmeticOutcome::CivilCandidate {
            date: date.parse().unwrap()
        }
    );
}
