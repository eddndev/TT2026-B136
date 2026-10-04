use super::*;
use application::case_stages::StageDocumentFormat;
use application::precautionary_hearings::PrecautionaryContext;
use application::typed_participants::{
    subject_digest, ParticipantCredentialRef, ParticipantText, RepresentedName, SubjectValues,
};
use domain::crypto::Sha256Digest;
use domain::hearings::HearingSupportRef;
use domain::identity::UserId;
use time::{Duration, UtcOffset};
use uuid::Uuid;

use crate::{context_support, measure_source_support, participant_support};

#[test]
fn result_source_inventory_rejects_missing_extra_duplicate_and_unselected_measure_ids() {
    for mutation in 0..5 {
        let mut fixture = Fixture::multiple();
        let sources = &mut fixture.material.result_sources;
        match mutation {
            0 => {
                sources.pop();
            }
            1 => {
                let mut extra = sources[0].clone();
                extra.id = id(90);
                sources.push(extra);
            }
            2 => sources[1] = sources[0].clone(),
            3 => sources[1].id = id(90),
            _ => sources.extend(vec![sources[0].clone(); 33]),
        }
        assert!(
            fixture.prepare().is_err(),
            "source inventory mutation {mutation}"
        );
    }
    let mut fixture = Fixture::no_change();
    fixture.material.result_sources = Fixture::single().material.result_sources;
    assert!(fixture.prepare().is_err());
}

#[test]
fn decision_preparation_revalidates_full_subject_and_supervisor_sources() {
    for mutation in 0..6 {
        let mut fixture = Fixture::single();
        let sources = &mut fixture.material.result_sources[0].sources;
        match mutation {
            0 => sources.subject.values_digest = Sha256Digest::from_array([99; 32]),
            1 => sources.supervisor = None,
            2 => {
                participant_support::typed_mut(sources.supervisor.as_mut().unwrap()).values_digest =
                    Sha256Digest::from_array([99; 32])
            }
            3 => sources.supervisor.as_mut().unwrap().bound_subject = None,
            4 => sources.subject.changed_by.email = "bad actor ".into(),
            _ => {
                sources.subject.changed_at = sources
                    .subject
                    .changed_at
                    .to_offset(UtcOffset::from_hms(1, 0, 0).unwrap())
            }
        }
        assert!(
            fixture.prepare().is_err(),
            "retained source mutation {mutation}"
        );
    }
}

#[test]
fn repeated_subject_revision_requires_identical_values_and_complete_provenance() {
    for mutation in 0..4 {
        let mut fixture = Fixture::multiple();
        let source = &mut fixture.material.result_sources[1].sources;
        match mutation {
            0 => source.subject.changed_by.id = UserId::from_uuid(Uuid::from_u128(9)),
            1 => source.subject.changed_by.email = "different@example.test".into(),
            2 => source.subject.changed_at += Duration::seconds(1),
            _ => {
                let SubjectValues::NaturalPerson { name, .. } = &mut source.subject.values else {
                    panic!("natural person fixture expected")
                };
                *name = RepresentedName::Known(
                    ParticipantText::new("Another retained person").unwrap(),
                );
                source.subject.values_digest = subject_digest(&Hasher, &source.subject.values);
            }
        }
        let values = MeasureValues::new(measure_source_support::input(source));
        resolve_measure_sources(&Hasher, fixture.case_id, &values, source).unwrap();
        fixture.replace_values(id(80), values);
        assert!(
            fixture.prepare().is_err(),
            "same subject revision mutation {mutation}"
        );
    }
}

#[test]
fn repeated_supervisor_revision_requires_full_provenance_and_typed_submission_material() {
    for mutation in 0..5 {
        let mut fixture = Fixture::multiple();
        fixture.material.result_sources[1].sources =
            fixture.material.result_sources[0].sources.clone();
        let source = &mut fixture.material.result_sources[1].sources;
        let snapshot = participant_support::typed_mut(source.supervisor.as_mut().unwrap());
        match mutation {
            0 => snapshot.changed_by.id = UserId::from_uuid(Uuid::from_u128(9)),
            1 => snapshot.changed_by.email = "another@example.test".into(),
            2 => snapshot.changed_at += Duration::seconds(1),
            3 => snapshot.submission_digest = Sha256Digest::from_array([99; 32]),
            _ => {
                snapshot.credential_origin = Some(ParticipantCredentialRef {
                    participant_id: snapshot.id,
                    participant_revision: snapshot.revision,
                    statement_digest: Sha256Digest::from_array([17; 32]),
                })
            }
        }
        let values = MeasureValues::new(measure_source_support::input(source));
        resolve_measure_sources(&Hasher, fixture.case_id, &values, source).unwrap();
        fixture.replace_values(id(80), values);
        assert!(
            fixture.prepare().is_err(),
            "same supervisor revision mutation {mutation}"
        );
    }
}

#[test]
fn one_participant_revision_cannot_be_both_manual_and_typed_across_results() {
    let mut fixture = Fixture::multiple();
    let source = &mut fixture.material.result_sources[1].sources;
    participant_support::manual_mut(source.supervisor.as_mut().unwrap()).id =
        participant_support::reference(7, 3).id();
    let values = MeasureValues::new(measure_source_support::input(source));
    resolve_measure_sources(&Hasher, fixture.case_id, &values, source).unwrap();
    fixture.replace_values(id(80), values);
    assert!(fixture.prepare().is_err());
}

#[test]
fn different_supervisors_cannot_supply_conflicting_copies_of_one_bound_subject_revision() {
    let mut fixture = Fixture::multiple();
    fixture.material.result_sources[1].sources = fixture.material.result_sources[0].sources.clone();
    let source = &mut fixture.material.result_sources[1].sources;
    let supervisor = source.supervisor.as_mut().unwrap();
    participant_support::typed_mut(supervisor).id = participant_support::reference(8, 3).id();
    supervisor.bound_subject.as_mut().unwrap().changed_by.email = "another@example.test".into();
    let values = MeasureValues::new(measure_source_support::input(source));
    resolve_measure_sources(&Hasher, fixture.case_id, &values, source).unwrap();
    fixture.replace_values(id(80), values);
    assert!(fixture.prepare().is_err());
}

fn shared_context_support() -> Fixture {
    let mut fixture = Fixture::single();
    let material = context_support::changed(context_support::intermediate());
    let support = match &material.stage {
        application::case_stages::CaseStageEntry::Changed(stage) => stage.supports[0].clone(),
        _ => panic!("changed stage fixture expected"),
    };
    fixture.material.context = PrecautionaryContext::new(&Hasher, material).unwrap();
    fixture.command.context = expectation(&fixture.material.context);
    fixture.material.support = support;
    let mut values = decision_input(&fixture.command.values);
    values.support = HearingSupportRef::new(
        fixture.material.support.reference,
        fixture.material.support.digest,
    );
    fixture.command.values = MeasureDecisionValues::new(values);
    fixture
}

#[test]
fn identical_decision_and_stage_support_metadata_can_share_one_exact_document_version() {
    measure_decision_group_matches(&Hasher, &shared_context_support().capture()).unwrap();
}

#[test]
fn same_document_version_cannot_have_conflicting_stage_and_decision_metadata() {
    for mutation in 0..3 {
        let mut fixture = shared_context_support();
        if mutation == 0 {
            fixture.material.support.name = "different-name.pdf".into();
        } else if mutation == 1 {
            fixture.material.support.format = StageDocumentFormat::Docx;
        } else {
            fixture.material.support.digest = Sha256Digest::from_array([99; 32]);
            let mut values = decision_input(&fixture.command.values);
            values.support = HearingSupportRef::new(
                fixture.material.support.reference,
                fixture.material.support.digest,
            );
            fixture.command.values = MeasureDecisionValues::new(values);
        }
        assert!(fixture.prepare().is_err());
    }
}
