use super::*;
use application::precautionary_hearings::{
    PrecautionaryHearingChange, PrecautionaryHearingCommand,
};
use domain::{
    hearings::{HearingModality, HearingTime, HearingVenue},
    precautionary_hearings::*,
};

#[test]
fn administrative_replay_retains_actual_precautionary_anchor_ancestry_after_hearing_changes() {
    let Some(mut db) = Fixture::new() else { return };
    let mut seed = crate::measure_fixture::setup(&mut db);
    let hearing = crate::hearing_fixture::persist(
        &db,
        seed.actor.clone(),
        PrecautionaryHearingCommand {
            operation_id: PrecautionaryHearingOperationId::new(),
            hearing_id: PrecautionaryHearingId::new(),
            change: PrecautionaryHearingChange::Schedule {
                context: seed.command.context,
                values: PrecautionaryHearingValues::new(PrecautionaryHearingValuesInput {
                    purpose: PrecautionaryHearingPurpose::Imposition,
                    scheduled_at: HearingTime::new(
                        (db.at + time::Duration::days(2))
                            .replace_nanosecond(0)
                            .unwrap(),
                    )
                    .unwrap(),
                    modality: HearingModality::InPerson,
                    venue: HearingVenue::new("Court for the declared measure hearing").unwrap(),
                    note: None,
                    participants: vec![],
                    scheduling_basis: PrecautionaryHearingSchedulingBasis::new(
                        note("Declared hearing appointment"),
                        seed.command.values.support(),
                        note("Page 1"),
                    ),
                    review_targets: vec![],
                })
                .unwrap(),
            },
        },
    );
    let selected = crate::hearing_fixture::persist(
        &db,
        seed.actor.clone(),
        crate::hearing_fixture::replacement(&hearing),
    );
    seed.command.anchor = Some(MeasureDecisionAnchorRef::Precautionary {
        hearing_id: selected.capture.review.command.hearing_id,
        revision: selected.capture.review.result_revision,
        capture_digest: selected.capture.capture_digest,
    });
    let judicial = crate::measure_fixture::persist(&db, seed.actor.clone(), seed.command.clone());
    let previous = &judicial.group.measures[0];
    let stored = persist(
        &db,
        seed.actor.clone(),
        correction(
            reference(previous),
            seed.command.context,
            &previous.result.values,
            "Corrected text retaining the anchored judicial decision",
        ),
    );
    let cancelled = crate::hearing_fixture::persist(
        &db,
        seed.actor.clone(),
        crate::hearing_fixture::cancellation(&selected),
    );

    assert_retained(&stored, &judicial, previous);
    assert_eq!(stored.record_history.records.judicial.groups.len(), 1);
    assert_eq!(
        stored.record_history.records.judicial.groups[0].capture,
        judicial.group
    );
    assert_eq!(
        judicial.group.review.material.anchor,
        Some(MeasureDecisionAnchorMaterial::Precautionary(Box::new(
            selected.capture
        )))
    );
    assert_eq!(cancelled.history.captures.len(), 3);
    reopened(&db, &seed.actor, &stored);
}
