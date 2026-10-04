use super::*;
use application::cases::{case_administration_digest, CaseAdministrativeStatus, CaseRevision};
use application::precautionary_hearings::PrecautionaryContext;
use domain::case_administration::CaseStageRevision;
use domain::cases::CaseId;
use domain::crypto::{DocumentId, DocumentVersion, Sha256Digest};
use domain::hearings::HearingNote;
use domain::identity::UserId;
use time::{Date, Duration, Month, UtcOffset};
use uuid::Uuid;

#[test]
fn preparation_requires_exact_case_and_every_expected_context_field() {
    for mutation in 0..4 {
        let mut fixture = Fixture::single();
        match mutation {
            0 => fixture.case_id = CaseId::from_uuid(Uuid::from_u128(99)),
            1 => fixture.command.context.administration_revision = CaseRevision::new(2).unwrap(),
            2 => fixture.command.context.stage_revision = CaseStageRevision::new(2).unwrap(),
            _ => fixture.command.context.context_digest = Sha256Digest::from_array([99; 32]),
        }
        assert!(fixture.prepare().is_err(), "context mutation {mutation}");
    }
}

#[test]
fn a_valid_historical_closed_context_cannot_prepare_a_new_decision() {
    let mut fixture = Fixture::single();
    let mut context = fixture.material.context.material().clone();
    context.administration.revision = CaseRevision::new(2).unwrap();
    context.administration.changed_at += Duration::seconds(1);
    context.administration.values = context
        .administration
        .values
        .with_status(CaseAdministrativeStatus::Closed);
    context.administration.values_digest =
        case_administration_digest(&Hasher, &context.administration.values);
    fixture.material.context = PrecautionaryContext::new(&Hasher, context).unwrap();
    fixture.command.context = expectation(&fixture.material.context);
    assert!(fixture.prepare().is_err());
}

#[test]
fn decision_support_requires_exact_identity_version_digest_and_safe_retained_name() {
    for mutation in 0..4 {
        let mut fixture = Fixture::single();
        let support = &mut fixture.material.support;
        match mutation {
            0 => support.reference.id = DocumentId::from_uuid(Uuid::from_u128(92)),
            1 => support.reference.version = DocumentVersion::new(2).unwrap(),
            2 => support.digest = Sha256Digest::from_array([12; 32]),
            _ => support.name = "../resolution.pdf".into(),
        }
        assert!(fixture.prepare().is_err(), "support mutation {mutation}");
    }
}

#[test]
fn preparation_rejects_malformed_actor_text_without_requiring_email_address_syntax() {
    for email in ["", " leading", "trailing ", "a\nb", "a\0b", "a\x7fb"] {
        let mut fixture = Fixture::single();
        fixture.actor.email = email.into();
        assert!(measure_decision_submission_bytes(
            &fixture.actor,
            fixture.case_id,
            &fixture.command
        )
        .is_err());
        assert!(fixture.prepare().is_err());
    }
    let mut fixture = Fixture::single();
    fixture.actor.email = "recorded account label".into();
    assert!(fixture.prepare().is_ok());
}

#[test]
fn each_instruction_dimension_changes_the_canonical_submission() {
    let baseline = Fixture::single();
    let bytes =
        measure_decision_submission_bytes(&baseline.actor, baseline.case_id, &baseline.command)
            .unwrap();
    for mutation in 0..17 {
        let mut fixture = baseline.clone();
        let mut values = decision_input(&fixture.command.values);
        match mutation {
            0 => fixture.actor.id = UserId::from_uuid(Uuid::from_u128(3)),
            1 => fixture.actor.email = "different@example.test".into(),
            2 => fixture.case_id = CaseId::from_uuid(Uuid::from_u128(2)),
            3 => {
                fixture.command.operation_id =
                    MeasureDecisionOperationId::from_uuid(Uuid::from_u128(101))
            }
            4 => fixture.command.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(111)),
            5 => fixture.command.context.administration_revision = CaseRevision::new(2).unwrap(),
            6 => fixture.command.context.stage_revision = CaseStageRevision::new(2).unwrap(),
            7 => fixture.command.context.context_digest = Sha256Digest::from_array([99; 32]),
            8 => values.authority = HearingNote::new("Different declared authority").unwrap(),
            9 => values.justification = HearingNote::new("Different justification").unwrap(),
            10 => values.locator = HearingNote::new("Page 3").unwrap(),
            11 => {
                values.declared_at = MeasureTime::new(
                    domain::procedural_time::DeclaredProceduralTime::unknown(),
                    Some(HearingNote::new("Time not legible").unwrap()),
                )
                .unwrap()
            }
            12 => fixture.command.outcome = Fixture::no_change().command.outcome,
            13 => {
                values.support = domain::hearings::HearingSupportRef::new(
                    domain::crypto::DocumentVersionRef {
                        id: DocumentId::from_uuid(Uuid::from_u128(92)),
                        version: values.support.reference().version,
                    },
                    values.support.digest(),
                )
            }
            14 => {
                values.support = domain::hearings::HearingSupportRef::new(
                    domain::crypto::DocumentVersionRef {
                        id: values.support.reference().id,
                        version: DocumentVersion::new(2).unwrap(),
                    },
                    values.support.digest(),
                )
            }
            15 => {
                values.support = domain::hearings::HearingSupportRef::new(
                    values.support.reference(),
                    Sha256Digest::from_array([99; 32]),
                )
            }
            _ => {
                let mut measure_values = crate::measure_source_support::input(
                    &fixture.material.result_sources[0].sources,
                );
                measure_values.conditions =
                    HearingNote::new("Different declared conditions").unwrap();
                fixture.replace_values(id(70), MeasureValues::new(measure_values));
            }
        }
        fixture.command.values = MeasureDecisionValues::new(values);
        let changed =
            measure_decision_submission_bytes(&fixture.actor, fixture.case_id, &fixture.command)
                .unwrap();
        assert_ne!(changed, bytes, "submission dimension {mutation}");
    }
}

#[test]
fn unsupported_existing_effects_are_rejected_even_with_a_real_predecessor_group() {
    let group = Fixture::single().capture();
    let previous = reference(&group.measures[0]);
    let values = group.measures[0].result.values.clone();
    let effects = [
        MeasureEffect::Confirm { previous },
        MeasureEffect::Modify {
            previous,
            values: values.clone(),
        },
        MeasureEffect::Revoke { previous },
        MeasureEffect::Cease { previous },
        MeasureEffect::Substitute {
            predecessors: vec![previous],
            successors: vec![MeasureProposal { id: id(80), values }],
        },
    ];
    for effect in effects {
        for with_sibling in [false, true] {
            let mut fixture = Fixture::single();
            let mut changes = vec![effect.clone()];
            if with_sibling {
                changes.push(MeasureEffect::Impose(MeasureProposal {
                    id: id(90),
                    values: group.measures[0].result.values.clone(),
                }));
            }
            fixture.command.outcome =
                MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(changes)).unwrap();
            fixture.command.operation_id =
                MeasureDecisionOperationId::from_uuid(Uuid::from_u128(101));
            fixture.command.decision_id = MeasureDecisionId::from_uuid(Uuid::from_u128(111));
            fixture.material.result_sources = fixture
                .command
                .outcome
                .affected_ids()
                .iter()
                .map(|id| MeasureResultSources {
                    id: *id,
                    sources: group.measures[0].result.sources.clone(),
                })
                .collect();
            fixture.material.predecessors = vec![owned(&group)];
            assert!(fixture.prepare().is_err());
        }
    }
}

#[test]
fn standalone_impose_and_no_change_reject_every_supplied_predecessor() {
    let group = Fixture::single().capture();
    for mut fixture in [Fixture::single(), Fixture::no_change()] {
        fixture.material.predecessors.push(owned(&group));
        assert!(fixture.prepare().is_err());
    }
}

#[test]
fn capture_clock_requires_supported_utc_and_cannot_predate_the_observed_context() {
    for timestamp in [
        at().to_offset(UtcOffset::from_hms(1, 0, 0).unwrap()),
        Date::from_calendar_date(0, Month::January, 1)
            .unwrap()
            .midnight()
            .assume_utc(),
        crate::context_support::at() - Duration::nanoseconds(1),
    ] {
        assert!(Fixture::single()
            .prepare()
            .unwrap()
            .into_group_capture(&Hasher, timestamp)
            .is_err());
    }
    Fixture::single()
        .prepare()
        .unwrap()
        .into_group_capture(&Hasher, crate::context_support::at())
        .unwrap();
}

#[test]
fn capture_clock_cannot_predate_any_retained_measure_source() {
    for location in 0..3 {
        let mut fixture = Fixture::single();
        let sources = &mut fixture.material.result_sources[0].sources;
        let future = at() + Duration::nanoseconds(1);
        match location {
            0 => sources.subject.changed_at = future,
            1 => {
                crate::participant_support::typed_mut(sources.supervisor.as_mut().unwrap())
                    .changed_at = future
            }
            _ => {
                sources
                    .supervisor
                    .as_mut()
                    .unwrap()
                    .bound_subject
                    .as_mut()
                    .unwrap()
                    .changed_at = future
            }
        }
        let checked = fixture.clone().prepare().unwrap();
        assert!(checked.into_group_capture(&Hasher, at()).is_err());
        fixture
            .prepare()
            .unwrap()
            .into_group_capture(&Hasher, future)
            .unwrap();
    }
    let mut fixture = Fixture::multiple();
    let sources = &mut fixture.material.result_sources[1].sources;
    crate::participant_support::manual_mut(sources.supervisor.as_mut().unwrap()).changed_at =
        at() + Duration::seconds(1);
    assert!(fixture
        .prepare()
        .unwrap()
        .into_group_capture(&Hasher, at())
        .is_err());
}

#[test]
fn declared_decision_and_measure_times_are_preserved_without_capture_clock_ordering() {
    use domain::procedural_time::DeclaredProceduralTime;
    let mut fixture = Fixture::single();
    let mut decision = decision_input(&fixture.command.values);
    decision.declared_at = MeasureTime::new(
        DeclaredProceduralTime::date("2030-01-01".parse().unwrap(), None).unwrap(),
        None,
    )
    .unwrap();
    fixture.command.values = MeasureDecisionValues::new(decision);
    let mut measure =
        crate::measure_source_support::input(&fixture.material.result_sources[0].sources);
    measure.validity = MeasureValidity::new(
        MeasureTime::new(
            DeclaredProceduralTime::date("2000-01-01".parse().unwrap(), None).unwrap(),
            None,
        )
        .unwrap(),
        HearingNote::new("Expressly declared validity").unwrap(),
        Some(
            MeasureTime::new(
                DeclaredProceduralTime::date("2050-01-01".parse().unwrap(), None).unwrap(),
                None,
            )
            .unwrap(),
        ),
    )
    .unwrap();
    let expected = MeasureValues::new(measure);
    fixture.replace_values(id(70), expected.clone());
    let group = fixture.capture();
    assert_eq!(group.measures[0].result.values, expected);
    assert_eq!(group.recorded_at, at());
    measure_decision_group_matches(&Hasher, &group).unwrap();
}
