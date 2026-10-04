use crate::deadline_support::evaluation::{self, inputs};
use crate::hearing_result_support as results;
use application::{
    deadline_inputs::DeadlineCalendarRef,
    deadline_profiles::*,
    deadline_tracking::{TrackingPolicies, TrackingPolicy},
    deadlines::*,
    hearing_derived_deadlines::*,
    hearing_results::*,
    identity::Principal,
    ApplicationError,
};
use domain::{
    cases::CaseId,
    deadline_arithmetic::{DayBasis, DayInclusion, FinalDayPolicy},
    deadline_profiles::{DeadlineRuleTemplate, OrderedDeadlineUnit},
    deadline_triggers::{TriggerField, TriggerRequirement, TriggerSourceRef},
    identity::Role,
    judicial_calendars::JudicialCalendarClassification,
    procedural_facts::{FactDeclaration, FactHearingRef},
};
use std::sync::Arc;
use time::{OffsetDateTime, UtcOffset};
use uuid::Uuid;

#[derive(Clone)]
pub struct Fixture {
    pub actor: Principal,
    pub result_preparation: HearingResultPreparation,
    pub command: HearingDerivedDeadlineCommand,
    pub material: HearingDerivedDeadlineMaterial,
}

pub fn case_id() -> CaseId {
    inputs::case_id()
}

pub fn agreement(value: u128) -> HearingResultAgreementId {
    HearingResultAgreementId::from_uuid(Uuid::from_u128(value))
}

pub fn fixture() -> Fixture {
    let actor = Principal {
        id: inputs::actor(),
        email: "owner@example.com".into(),
        role: Role::Owner,
    };
    let result_preparation = results::preparation(case_id(), actor.id);
    let mut result = results::command(&result_preparation);
    let mut values = results::values_input(&results::values());
    values.event_time =
        DeclaredHearingResultTime::date(evaluation::date("2026-01-06").date(), UtcOffset::UTC)
            .unwrap();
    values.agreements = vec![
        HearingResultAgreement::new(
            agreement(501),
            HearingResultText::new("First declared agreement").unwrap(),
        ),
        HearingResultAgreement::new(
            agreement(502),
            HearingResultText::new("Second declared agreement").unwrap(),
        ),
    ];
    let HearingResultChange::Record { values: target, .. } = &mut result.change else {
        unreachable!()
    };
    *target = HearingResultValues::new(values).unwrap();
    let result_draft = prepare_result(&actor, &result_preparation, result.clone());
    let (mut deadline, mut existing) = crate::deadline_support::fixture();
    let mut profile = existing.resolved.take().unwrap().profile;
    let calendar = inputs::calendar(1, false, JudicialCalendarClassification::Countable);
    let mut definition = evaluation::definition();
    evaluation::ordered(&mut definition);
    definition.trigger = TriggerRequirement::SourceField(TriggerField::HearingSessionEventTime);
    definition.template = DeadlineRuleTemplate::Ordered {
        unit: OrderedDeadlineUnit::Days {
            inclusion: DayInclusion::OnAnchor,
            basis: DayBasis::CalendarCountable,
            final_day: FinalDayPolicy::Preserve,
        },
        maximum: Some(evaluation::n(6)),
    };
    definition.examples[0].calendar = Some(calendar.values.clone());
    profile.definition = DeadlineProfileDefinition::new(definition).unwrap();
    resign_profile(&mut profile);
    let DeadlineChange::Register { definition } = &mut deadline.change else {
        unreachable!()
    };
    definition.input.selection.source =
        FactDeclaration::Known(TriggerSourceRef::HearingResult(FactHearingRef {
            hearing_id: result.hearing_id,
            result_id: result.result_id,
            revision: HearingResultRevision::initial(),
            agreement_id: Some(agreement(501)),
        }));
    definition.input.ordered_quantity = Some(evaluation::n(2));
    definition.input.calendar = Some(DeadlineCalendarRef {
        id: calendar.id,
        revision: calendar.revision,
    });
    let policies = TrackingPolicies {
        profile: TrackingPolicy::Fixed,
        source: TrackingPolicy::Fixed,
        calendar: TrackingPolicy::Fixed,
    };
    Fixture {
        actor,
        result_preparation,
        command: HearingDerivedDeadlineCommand {
            result,
            deadline: DeadlineHumanCommand::new(deadline, Some(policies)).unwrap(),
        },
        material: HearingDerivedDeadlineMaterial {
            result: result_draft,
            profile: profile.clone(),
            profile_head: profile,
            calendar: Some(calendar.clone()),
            calendar_head: Some(calendar),
            responsible: existing.responsible.unwrap(),
        },
    }
}

fn prepare_result(
    actor: &Principal,
    preparation: &HearingResultPreparation,
    command: HearingResultCommand,
) -> HearingResultDraft {
    assert!(preparation.base.is_none());
    let mut store = results::MockStore::new();
    store.expect_get().never();
    store.expect_commit().never();
    let expected = command.clone();
    let owner = actor.id;
    store
        .expect_prepare()
        .withf(move |id, case, input, _| *id == owner && *case == case_id() && *input == expected)
        .times(1)
        .return_once({
            let preparation = preparation.clone();
            move |_, _, _, _| Ok(preparation)
        });
    let mut identity = results::MockIdentity::new();
    let actor = actor.clone();
    identity
        .expect_authenticate()
        .times(2)
        .returning(move |_| Ok(actor.clone()));
    let (service, clock) =
        results::service(store, identity, Arc::new(results::Validator::default()));
    let draft = service.prepare("session", case_id(), command).unwrap();
    assert_eq!(clock.calls(), 1);
    draft
}

impl Fixture {
    pub fn prepare(&self) -> Result<HearingDerivedDeadlineDraft, ApplicationError> {
        self.prepare_at(crate::case_support::instant())
    }

    pub fn prepare_at(
        &self,
        observed_at: OffsetDateTime,
    ) -> Result<HearingDerivedDeadlineDraft, ApplicationError> {
        prepare_hearing_derived_deadline(
            results::hasher().as_ref(),
            &self.actor,
            case_id(),
            self.command.clone(),
            self.material.clone(),
            observed_at,
        )
    }

    pub fn edit_deadline(
        &mut self,
        edit: impl FnOnce(&mut DeadlineCommand, &mut TrackingPolicies),
    ) {
        let (mut command, policies) = self.command.deadline.clone().into_parts();
        let mut policies = policies.unwrap();
        edit(&mut command, &mut policies);
        self.command.deadline = DeadlineHumanCommand::new(command, Some(policies)).unwrap();
    }

    pub fn edit_definition(&mut self, edit: impl FnOnce(&mut DeadlineDefinition)) {
        self.edit_deadline(|command, _| {
            let DeadlineChange::Register { definition } = &mut command.change else {
                panic!("registration expected")
            };
            edit(definition);
        });
    }

    pub fn edit_values(&mut self, edit: impl FnOnce(&mut HearingResultValuesInput)) {
        let HearingResultChange::Record { values, .. } = &mut self.command.result.change else {
            panic!("result registration expected")
        };
        let mut input = results::values_input(values);
        edit(&mut input);
        *values = HearingResultValues::new(input).unwrap();
        self.refresh_result();
    }

    pub fn refresh_result(&mut self) {
        self.material.result = prepare_result(
            &self.actor,
            &self.result_preparation,
            self.command.result.clone(),
        );
    }

    pub fn edit_profile(&mut self, edit: impl FnOnce(&mut DeadlineProfileDefinitionInput)) {
        let mut input = profile_input(&self.material.profile);
        edit(&mut input);
        self.material.profile.definition = DeadlineProfileDefinition::new(input).unwrap();
        resign_profile(&mut self.material.profile);
        self.material.profile_head = self.material.profile.clone();
    }
}

pub fn profile_input(profile: &DeadlineProfileDetail) -> DeadlineProfileDefinitionInput {
    let value = &profile.definition;
    DeadlineProfileDefinitionInput {
        title: value.title().clone(),
        description: value.description().clone(),
        scope: value.scope().clone(),
        references: value.references().to_vec(),
        trigger: value.trigger(),
        template: value.template(),
        completion: value.completion().clone(),
        conditions: value.conditions().to_vec(),
        examples: value.examples().to_vec(),
    }
}

pub fn resign_profile(value: &mut DeadlineProfileDetail) {
    let change = if value.revision == DeadlineProfileRevision::initial() {
        DeadlineProfileChange::Publish {
            definition: value.definition.clone(),
        }
    } else {
        DeadlineProfileChange::Replace {
            expected_revision: DeadlineProfileRevision::new(value.revision.get() - 1).unwrap(),
            definition: value.definition.clone(),
            reason: value.reason.clone().unwrap(),
        }
    };
    value.definition_digest =
        deadline_profile_definition_digest(results::hasher().as_ref(), &value.definition);
    let command = DeadlineProfileCommand {
        operation_id: value.receipt.operation_id,
        profile_id: value.id,
        change,
    };
    value.receipt.submission_digest = deadline_profile_submission_digest(
        results::hasher().as_ref(),
        value.recorded_by.id,
        &command,
        value.algorithm,
        value.definition_digest,
    );
    deadline_profile_receipt_matches(results::hasher().as_ref(), value).unwrap();
}
