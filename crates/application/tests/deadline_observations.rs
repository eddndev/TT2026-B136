#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_observation_support;
mod deadline_support;
use deadline_observation_support::*;

use application::{
    cases::CurrentCaseAdministration, deadline_inputs::DeadlineSourceDetail,
    deadline_observations::build_deadline_observations, deadline_profiles::*,
    deadline_reevaluation::*, procedural_facts::*,
};
use domain::{
    cases::CaseId,
    crypto::Sha256Digest,
    deadline_triggers::{TriggerField, TriggerRequirement},
    judicial_calendars::JudicialCalendarClassification,
};
use uuid::Uuid;

#[test]
fn builds_canonical_head_observations_without_replacing_selected_history() {
    let (mut profile, mut material) = fixture();
    replace_profile(&mut profile, false);
    material.source_head = Some(DeadlineSourceDetail::Fact(Box::new(inputs::resolution(
        3,
        false,
        "2026-01-09",
    ))));
    calendar_pair(&mut material);
    let before = (profile.clone(), material.clone());
    let observed = build(&profile, &material, None).unwrap();
    assert_eq!(observed.case_id, inputs::case_id());
    assert_eq!(
        observed.entries.iter().map(|v| v.role).collect::<Vec<_>>(),
        [
            ObservationRole::Profile,
            ObservationRole::Source,
            ObservationRole::Calendar
        ]
    );
    assert_eq!(
        observed
            .entries
            .iter()
            .map(|v| v.revision)
            .collect::<Vec<_>>(),
        [2, 3, 2]
    );
    assert_eq!(observed.entries[0].id, profile.id.as_uuid());
    assert_eq!(observed.entries[0].case_id, Some(inputs::case_id()));
    assert_eq!(observed.entries[1].family, DependencyFamily::Resolution);
    assert_eq!(
        observed.entries[1].submission_digest,
        fact(&material.source_head)
            .snapshot
            .metadata()
            .receipt
            .submission_digest
    );
    assert_eq!(observed.entries[2].case_id, None);
    assert_eq!(
        decode_observations(&encode_observations(&observed).unwrap()).unwrap(),
        observed
    );
    assert_eq!((profile, material), before);
}

#[test]
fn observed_profile_need_not_qualify_the_preserved_source_family() {
    let (mut profile, material) = fixture();
    let mut input = profile_input(&profile);
    input.trigger = TriggerRequirement::SourceField(TriggerField::NotificationPracticedAt);
    profile.definition = DeadlineProfileDefinition::new(input).unwrap();
    replace_profile(&mut profile, false);
    assert_eq!(build(&profile, &material, None).unwrap().entries.len(), 2);
}

#[test]
fn a_profile_and_optional_calendar_can_be_observed_without_a_known_source() {
    let (profile, _) = fixture();
    let (_, mut material) = inputs::unknown();
    assert_eq!(build(&profile, &material, None).unwrap().entries.len(), 1);
    calendar_pair(&mut material);
    let observed = build(&profile, &material, None).unwrap();
    assert_eq!(observed.entries[1].role, ObservationRole::Calendar);
}

#[test]
fn valid_retirements_and_closed_administration_remain_observable() {
    let (mut profile, mut material) = fixture();
    replace_profile(&mut profile, true);
    material.administration = administration(inputs::case_id(), true);
    material.source_head = Some(DeadlineSourceDetail::Fact(Box::new(inputs::resolution(
        2,
        true,
        "2026-01-06",
    ))));
    calendar_pair(&mut material);
    material.calendar_head = Some(inputs::calendar(
        2,
        true,
        JudicialCalendarClassification::Countable,
    ));
    let observed = build(&profile, &material, None).unwrap();
    assert_eq!(observed.entries.len(), 3);
    assert_eq!(
        observed
            .entries
            .iter()
            .map(|v| v.revision)
            .collect::<Vec<_>>(),
        [2, 2, 2]
    );
}

#[test]
fn notification_keeps_both_exact_parents_separate_from_observed_parent_head() {
    let (profile, _) = fixture();
    let (material, parent) = notification_material();
    let before = material.clone();
    let observed = build(&profile, &material, Some(&parent)).unwrap();
    let source = entry(&observed, ObservationRole::Source);
    let related = entry(&observed, ObservationRole::NotificationParent);
    assert_eq!(source.parent_resolution.unwrap().revision, 2);
    assert_eq!(related.revision, 3);
    assert_eq!(source.parent_resolution.unwrap().id, related.id);
    assert_eq!(related.family, DependencyFamily::Resolution);
    assert_eq!(
        fact(&material.source)
            .sources
            .resolved
            .resolution
            .unwrap()
            .reference
            .revision
            .get(),
        1
    );
    assert_eq!(material, before);
}

#[test]
fn notification_parent_must_cover_both_selected_and_head_parent_revisions() {
    let (profile, _) = fixture();
    let (mut material, _) = notification_material();
    for swap in [false, true] {
        if swap {
            material.source = Some(DeadlineSourceDetail::Fact(Box::new(inputs::notification(
                1,
                false,
                3,
                "2026-01-06",
            ))));
            material.source_head = Some(DeadlineSourceDetail::Fact(Box::new(
                inputs::notification(2, false, 1, "2026-01-07"),
            )));
        }
        let old = inputs::resolution(1, false, "2026-01-01");
        assert!(build(&profile, &material, Some(&old)).is_err());
        let current = inputs::resolution(3, false, "2026-01-01");
        assert!(build(&profile, &material, Some(&current)).is_ok());
    }
}

#[test]
fn a_result_observation_has_its_hearing_root_and_never_selects_an_agreement() {
    let (profile, _) = fixture();
    let selected = DeadlineSourceDetail::HearingResult(Box::new(inputs::hearing(
        1,
        false,
        "2026-01-06",
        &[700],
    )));
    let mut material = inputs::material(selected);
    let head = inputs::hearing(2, false, "2026-01-07", &[701, 702]);
    material.source_head = Some(DeadlineSourceDetail::HearingResult(Box::new(head.clone())));
    let observed = build(&profile, &material, None).unwrap();
    let source = entry(&observed, ObservationRole::Source);
    assert_eq!(source.family, DependencyFamily::HearingResult);
    assert_eq!(source.id, head.snapshot.id.as_uuid());
    assert_eq!(source.hearing_id, Some(head.snapshot.hearing_id.as_uuid()));
    assert_eq!(source.revision, 2);
    assert_eq!(source.parent_resolution, None);
}

#[test]
fn global_profiles_are_observed_without_a_case_scope_projection() {
    let (mut profile, material) = fixture();
    let mut input = profile_input(&profile);
    input.scope = DeadlineProfileScope::Global(
        inputs::calendar(1, false, JudicialCalendarClassification::Countable)
            .values
            .scope()
            .clone(),
    );
    profile.definition = DeadlineProfileDefinition::new(input).unwrap();
    resign_profile(&mut profile);
    assert_eq!(
        build(&profile, &material, None).unwrap().entries[0].case_id,
        None
    );
}

#[test]
fn nil_case_and_dependency_identities_are_not_treated_as_absence() {
    let (mut profile, mut material) = fixture();
    let case = CaseId::from_uuid(Uuid::nil());
    let mut input = profile_input(&profile);
    input.scope = DeadlineProfileScope::Case(case);
    profile.definition = DeadlineProfileDefinition::new(input).unwrap();
    profile.id = DeadlineProfileId::from_uuid(Uuid::nil());
    resign_profile(&mut profile);
    material.case_id = case;
    let source = fact_mut(&mut material.source);
    let ProceduralFactSnapshot::Resolution(snapshot) = &mut source.snapshot else {
        unreachable!()
    };
    snapshot.root = ResolutionRoot::new(ResolutionId::from_uuid(Uuid::nil()), case);
    inputs::resign_fact(source);
    material.source_head = material.source.clone();
    let observed =
        build_deadline_observations(inputs::hasher().as_ref(), case, &profile, &material, None)
            .unwrap();
    assert_eq!(observed.case_id, case);
    assert!(observed.entries.iter().all(|entry| entry.id.is_nil()));
    assert_eq!(observed.entries[1].case_id, Some(case));
}

#[test]
fn cross_case_or_corrupt_administration_is_rejected_even_without_a_source() {
    let (profile, _) = fixture();
    for kind in 0..3 {
        let (_, mut material) = inputs::unknown();
        if kind == 0 {
            material.case_id = CaseId::from_uuid(Uuid::nil());
        } else {
            material.administration = administration(
                if kind == 1 {
                    CaseId::from_uuid(Uuid::nil())
                } else {
                    inputs::case_id()
                },
                true,
            );
            if kind == 2 {
                let CurrentCaseAdministration::Recorded(snapshot) = &mut material.administration
                else {
                    unreachable!()
                };
                snapshot.values_digest = Sha256Digest::from_array([0; 32]);
            }
        }
        assert!(build(&profile, &material, None).is_err());
    }
}
