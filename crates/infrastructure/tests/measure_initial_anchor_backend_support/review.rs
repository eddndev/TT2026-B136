use super::*;
use domain::{
    hearings::{HearingModality, HearingTime, HearingVenue},
    precautionary_hearings::*,
};

#[test]
fn precautionary_review_retains_the_real_owning_groups_full_initial_hearing_anchor() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let group = persist(&db, seed.actor.clone(), seed.command.clone());
    let measure = &group.group.measures[0];
    let values = PrecautionaryHearingValues::new(PrecautionaryHearingValuesInput {
        purpose: PrecautionaryHearingPurpose::Review,
        scheduled_at: HearingTime::new(
            (db.at + time::Duration::days(2))
                .replace_nanosecond(0)
                .unwrap(),
        )
        .unwrap(),
        modality: HearingModality::InPerson,
        venue: HearingVenue::new("Court for the declared review").unwrap(),
        note: None,
        participants: vec![],
        scheduling_basis: PrecautionaryHearingSchedulingBasis::new(
            note("Declared review appointment"),
            group.group.review.command.values.support(),
            note("Page 1"),
        ),
        review_targets: vec![PrecautionaryMeasureRef::new(
            measure.result.id,
            measure.result.revision,
            measure.capture_digest,
        )],
    })
    .unwrap();
    let command = PrecautionaryHearingCommand {
        operation_id: PrecautionaryHearingOperationId::new(),
        hearing_id: PrecautionaryHearingId::new(),
        change: PrecautionaryHearingChange::Schedule {
            context: seed.command.context,
            values,
        },
    };

    let hearing = crate::precautionary_fixture::persist(&db, seed.actor.clone(), command.clone());

    assert_eq!(hearing.history.measure_history.groups.len(), 1);
    let owner = &hearing.history.measure_history.groups[0];
    assert_eq!(owner.origin, group.origin);
    assert_eq!(owner.capture, group.group);
    assert_eq!(
        owner.capture.review.material.anchor,
        expected_anchor(&seed.hearing)
    );
    let query = crate::precautionary_fixture::reads(&db, seed.actor);
    assert_eq!(
        query
            .get("session", db.case, command.hearing_id, None)
            .unwrap(),
        hearing
    );
    assert_eq!(
        query
            .get_operation("session", db.case, command.operation_id)
            .unwrap(),
        hearing
    );
}
