use crate::{deadline_http_support as deadlines, hearing_result_support as results};
use application::{
    deadline_profiles::DeadlineProfileDefinition,
    deadline_reevaluation::{DependencyFamily, SourceEventReference},
    deadlines::*,
    hearing_derived_deadlines::*,
    hearing_results::*,
    identity::Principal,
};
use deadlines::records::Hasher;
use domain::{
    deadline_triggers::{TriggerField, TriggerRequirement, TriggerSourceRef},
    identity::Role,
    procedural_facts::{FactDeclaration, FactHearingRef},
};
use time::{Date, Month, OffsetDateTime};
use uuid::Uuid;

pub const EVENT_SEQUENCE: u64 = 9_007_199_254_740_993;

pub fn now() -> OffsetDateTime {
    Date::from_calendar_date(2026, Month::October, 4)
        .unwrap()
        .with_hms(12, 0, 0)
        .unwrap()
        .assume_utc()
}

pub fn fixture() -> (HearingDerivedDeadlineDraft, HearingDerivedDeadlineRecord) {
    fixture_with_time(results::values().event_time())
}

pub fn fixture_with_time(
    event_time: DeclaredHearingResultTime,
) -> (HearingDerivedDeadlineDraft, HearingDerivedDeadlineRecord) {
    let seed = deadlines::records::fixture();
    let actor = Principal {
        id: deadlines::records::actor(),
        email: "owner@example.com".into(),
        role: Role::Owner,
    };
    let mut result_command = results::command();
    let HearingResultChange::Record { values, .. } = &mut result_command.change else {
        unreachable!()
    };
    *values = HearingResultValues::new(HearingResultValuesInput {
        occurrence: values.occurrence(),
        extent: values.extent(),
        event_time,
        summary: values.summary().clone(),
        attendees: vec![],
        agreements: vec![],
        provenance: values.provenance().clone(),
    })
    .unwrap();
    let mut detail = results::detail(&result_command);
    detail.snapshot.values_digest = deadlines::digest();
    detail.snapshot.receipt.submission_digest = deadlines::digest();
    detail.snapshot.recorded_administration_digest = deadlines::digest();
    detail.snapshot.recorded_at = now();
    detail.snapshot.recorded_by.id = actor.id;
    detail.snapshot.recorded_by.email = actor.email.clone();
    detail.anchor.reference.values_digest = deadlines::digest();
    detail.anchor.reference.submission_digest = deadlines::digest();
    detail.anchor.scheduling_context.administration_digest = deadlines::digest();
    detail.snapshot.anchor = detail.anchor.reference;
    let mut administration = results::administration();
    let application::cases::CurrentCaseAdministration::Recorded(ref mut admin) = administration
    else {
        unreachable!()
    };
    admin.values_digest = deadlines::digest();
    let result = HearingResultDraft {
        case_id: results::case(),
        actor: actor.id,
        command: result_command.clone(),
        result_revision: HearingResultRevision::initial(),
        values: detail.snapshot.values.clone(),
        values_digest: deadlines::digest(),
        submission_digest: deadlines::digest(),
        anchor: detail.anchor,
        continuation: None,
        observed_administration: administration,
        attendees: vec![],
        support: None,
    };
    let mut profile = seed.calculation.profile;
    let mut p = deadlines::records::profiles::input(Some(results::case()));
    p.conditions[0].id = Uuid::nil();
    p.trigger = TriggerRequirement::SourceField(TriggerField::HearingSessionEventTime);
    profile.definition = DeadlineProfileDefinition::new(p).unwrap();
    let mut definition = seed.definition;
    definition.input.selection.case_id = results::case();
    definition.input.selection.source =
        FactDeclaration::Known(TriggerSourceRef::HearingResult(FactHearingRef {
            hearing_id: result_command.hearing_id,
            result_id: result_command.result_id,
            revision: HearingResultRevision::initial(),
            agreement_id: None,
        }));
    let command = HearingDerivedDeadlineCommand {
        result: result_command,
        deadline: DeadlineHumanCommand::new(
            DeadlineCommand {
                operation_id: seed.receipt.operation_id,
                deadline_id: seed.id,
                change: DeadlineChange::Register { definition },
            },
            Some(deadlines::records::tracking_policies()),
        )
        .unwrap(),
    };
    let material = HearingDerivedDeadlineMaterial {
        result,
        profile: profile.clone(),
        profile_head: profile,
        calendar: None,
        calendar_head: None,
        responsible: seed.responsible,
    };
    let draft = prepare_hearing_derived_deadline(
        &Hasher,
        &actor,
        results::case(),
        command,
        material,
        now(),
    )
    .expect("coherent prospective HTTP fixture");
    let source_event = SourceEventReference {
        sequence: EVENT_SEQUENCE,
        family: DependencyFamily::HearingResult,
        source_id: detail.snapshot.id.as_uuid(),
        revision: 1,
        case_id: Some(results::case()),
        hearing_id: Some(detail.snapshot.hearing_id.as_uuid()),
        operation_id: detail.snapshot.receipt.operation_id.as_uuid(),
    };
    let creation = finalize_hearing_derived_deadline(&Hasher, &draft, detail, source_event)
        .expect("coherent final capture HTTP fixture");
    let record = restore_hearing_derived_deadline(&Hasher, creation.evidence())
        .expect("coherent historical HTTP fixture");
    (draft, record)
}
