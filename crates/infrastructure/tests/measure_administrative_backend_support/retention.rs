use super::*;
use application::participants::{DirectoryStatus, ParticipantStore};
use domain::{
    crypto::DocumentVersionRef,
    hearings::{HearingParticipantRef, HearingSupportRef},
};
use infrastructure::PostgresParticipantStore;

#[test]
fn correction_retains_the_last_actual_judicial_support_after_a_modification() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, first, _) = setup(&mut db);
    let previous = &first.group.measures[0];
    let old = &previous.result.values;
    let changed = MeasureValues::new(MeasureValuesInput {
        subject: old.subject(),
        kind: old.kind(),
        conditions: note("Judicially modified terms"),
        validity: old.validity().clone(),
        supervision: old.supervision().clone(),
    });
    let mut command = effect_command(
        &seed.command,
        vec![MeasureEffect::Modify {
            previous: reference(previous),
            values: changed,
        }],
    );
    let document = crate::measure_fixture::upload(&db, db.case, "later-measure-support.pdf");
    command.values = MeasureDecisionValues::new(MeasureDecisionValuesInput {
        authority: note("Declared court"),
        declared_at: crate::measure_fixture::declared_time(),
        justification: note("Declared modification reasons"),
        locator: note("Page 2"),
        support: HearingSupportRef::new(
            DocumentVersionRef {
                id: document.id,
                version: document.version,
            },
            document.digest,
        ),
    });
    let modified = crate::measure_fixture::persist(&db, seed.actor.clone(), command);
    let selected = &modified.group.measures[0];
    let stored = persist(
        &db,
        seed.actor.clone(),
        correction(
            reference(selected),
            seed.command.context,
            &selected.result.values,
            "Corrected transcription of the modified terms",
        ),
    );

    assert_retained(&stored, &modified, selected);
    assert_eq!(stored.capture.review.support.reference.id, document.id);
    assert_ne!(
        stored.capture.review.support.reference.id,
        first.group.decision.support.reference.id
    );
    assert_eq!(stored.record_history.records.judicial.groups.len(), 2);
    reopened(&db, &seed.actor, &stored);
}

#[test]
fn correction_of_a_terminal_declaration_preserves_its_judicial_action() {
    let Some(mut db) = Fixture::new() else { return };
    let (seed, first, _) = setup(&mut db);
    let revoked = crate::measure_fixture::persist(
        &db,
        seed.actor.clone(),
        effect_command(
            &seed.command,
            vec![MeasureEffect::Revoke {
                previous: reference(&first.group.measures[0]),
            }],
        ),
    );
    let selected = &revoked.group.measures[0];
    let stored = persist(
        &db,
        seed.actor.clone(),
        correction(
            reference(selected),
            seed.command.context,
            &selected.result.values,
            "Corrected transcription retained after revocation",
        ),
    );

    assert_retained(&stored, &revoked, selected);
    assert_eq!(
        stored.capture.review.result.last_action,
        MeasureCaptureAction::Revoke
    );
    assert_eq!(
        stored.capture.review.result.validity,
        MeasureCaptureValidity::Valid
    );
    assert_eq!(
        stored.capture.review.result.values.validity(),
        selected.result.values.validity()
    );
    reopened(&db, &seed.actor, &stored);
}

#[test]
fn correction_preserves_the_exact_historical_supervisor_after_archiving() {
    let Some(mut db) = Fixture::new() else { return };
    let mut seed = crate::measure_fixture::setup(&mut db);
    let old = crate::measure_fixture::values(&seed.subject);
    let values = MeasureValues::new(MeasureValuesInput {
        subject: old.subject(),
        kind: old.kind(),
        conditions: old.conditions().clone(),
        validity: old.validity().clone(),
        supervision: MeasureSupervision::Known {
            participant: HearingParticipantRef::new(
                seed.supervisor.id(),
                seed.supervisor.revision_number(),
            ),
            statement: note("Declared exact historical supervisor"),
        },
    });
    seed.command.outcome = MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(vec![
        MeasureEffect::Impose(MeasureProposal {
            id: domain::precautionary_hearings::MeasureId::new(),
            values,
        }),
    ]))
    .unwrap();
    let judicial = crate::measure_fixture::persist(&db, seed.actor.clone(), seed.command.clone());
    let participants =
        PostgresParticipantStore::open(&db.runtime_url, Arc::new(RingSha256Hasher)).unwrap();
    let archived = participants
        .change_status(
            db.owner,
            db.case,
            seed.supervisor.id(),
            seed.supervisor.revision_number(),
            DirectoryStatus::Archived,
            db.at,
        )
        .unwrap();
    assert_ne!(
        archived.revision_number(),
        seed.supervisor.revision_number()
    );
    let previous = &judicial.group.measures[0];
    let stored = persist(
        &db,
        seed.actor.clone(),
        correction(
            reference(previous),
            seed.command.context,
            &previous.result.values,
            "Corrected text with retained historical supervision",
        ),
    );

    assert_retained(&stored, &judicial, previous);
    assert_eq!(
        stored.capture.review.result.sources.supervisor.as_ref(),
        Some(&seed.supervisor)
    );
    reopened(&db, &seed.actor, &stored);
}
