use super::*;
use application::{cases::CaseRevision, precautionary_hearings::PrecautionaryContext};

fn advanced() -> (RecordFixture, MeasureAdministrativeCapture) {
    let mut fixture = RecordFixture::initial();
    let mut context = fixture.context.material().clone();
    context.administration.revision = CaseRevision::new(2).unwrap();
    context.administration.changed_at = fixture.recorded_at - Duration::seconds(1);
    fixture.context = PrecautionaryContext::new(&Hasher, context).unwrap();
    fixture.command.context = expectation(&fixture.context);
    let capture = fixture.capture();
    (fixture, capture)
}

#[test]
fn v2_context_advances_from_selected_correction_instead_of_last_judicial_capture() {
    let (prior_fixture, prior) = advanced();
    for mutation in 0..3 {
        let mut fixture = FixtureV2::confirm(&prior, &prior_fixture.history);
        let mut context = if mutation == 0 {
            crate::context_support::initial()
        } else {
            fixture.material.context.material().clone()
        };
        match mutation {
            1 => {
                context.administration.revision = CaseRevision::new(3).unwrap();
                context.administration.changed_at -= Duration::seconds(1);
            }
            2 => {
                context.administration.changed_by.email =
                    "different historical administrator".into()
            }
            _ => {}
        }
        fixture.material.context = PrecautionaryContext::new(&Hasher, context).unwrap();
        fixture.command.context = expectation(&fixture.material.context);
        assert!(fixture.prepare().is_err());
    }
}

#[test]
fn v2_candidate_cannot_rewrite_stage_administration_only_retained_in_ancestor_contexts() {
    let (prior_fixture, prior) = corrected();
    let mut fixture = FixtureV2::confirm(&prior, &prior_fixture.history);
    let mut context = crate::context_support::initial();
    context.administration.revision = CaseRevision::new(2).unwrap();
    context.administration.changed_at += Duration::seconds(1);
    context.stage_administration.changed_by.email = "different stage registrar".into();
    let application::case_stages::CaseStageEntry::Initial(stage) = &mut context.stage else {
        unreachable!()
    };
    stage.recorded_by = context.stage_administration.changed_by.clone();
    fixture.material.context = PrecautionaryContext::new(&Hasher, context).unwrap();
    fixture.command.context = expectation(&fixture.material.context);
    assert!(fixture.prepare().is_err());
}

#[test]
fn v2_modify_cannot_reuse_retained_source_revision_with_changed_full_provenance() {
    let (prior_fixture, prior) = corrected();
    let mut fixture = FixtureV2::confirm(&prior, &prior_fixture.history);
    let mut values = crate::effect_support::values_input(&prior.review.result.values);
    values.conditions = note("New declared judicial terms");
    effects(
        &mut fixture,
        vec![MeasureEffect::Modify {
            previous: record_reference(&prior.records[0]),
            values: MeasureValues::new(values),
        }],
    );
    let sources = &mut fixture.material.result_sources[0].sources;
    crate::participant_support::typed_mut(sources.supervisor.as_mut().unwrap())
        .changed_by
        .email = "different retained supervisor recorder".into();
    assert!(fixture.prepare().is_err());
}
