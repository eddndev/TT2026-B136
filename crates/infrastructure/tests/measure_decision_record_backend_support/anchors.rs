use super::*;
use application::precautionary_hearings::*;
use domain::hearings::{HearingModality, HearingTime, HearingVenue};
use domain::precautionary_hearings::*;

pub(super) fn hearing(
    db: &Fixture,
    seed: &RecordSeed,
    target: PrecautionaryMeasureRef,
) -> PrecautionaryHearingRecordStoredOperation {
    let command = PrecautionaryHearingCommand {
        operation_id: PrecautionaryHearingOperationId::new(),
        hearing_id: PrecautionaryHearingId::new(),
        change: PrecautionaryHearingChange::Schedule {
            context: seed.command.context,
            values: PrecautionaryHearingValues::new(PrecautionaryHearingValuesInput {
                purpose: PrecautionaryHearingPurpose::Review,
                scheduled_at: HearingTime::new(
                    (db.at + time::Duration::days(2))
                        .replace_nanosecond(0)
                        .unwrap(),
                )
                .unwrap(),
                modality: HearingModality::InPerson,
                venue: HearingVenue::new("Court for the mixed-record decision").unwrap(),
                note: None,
                participants: vec![],
                scheduling_basis: PrecautionaryHearingSchedulingBasis::new(
                    note("Declared mixed-record review"),
                    seed.command.values.support(),
                    note("Page 1"),
                ),
                review_targets: vec![target],
            })
            .unwrap(),
        },
    };
    let workflow = PrecautionaryHearingRecordService::new(
        crate::hearing_fixture::store(db),
        Arc::new(TestIdentity(seed.actor.clone())),
        processor(),
        Arc::new(FormatCheck(None)),
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    );
    let review = workflow
        .prepare("session", db.case, command.clone())
        .unwrap();
    workflow
        .submit(
            "session",
            db.case,
            command,
            crate::hearing_fixture::confirmation(&review),
        )
        .unwrap()
}

pub(super) fn attach(
    command: &mut MeasureDecisionCommand,
    hearing: &PrecautionaryHearingRecordStoredOperation,
) {
    command.anchor = Some(MeasureDecisionAnchorRef::Precautionary {
        hearing_id: hearing.capture.review.command.hearing_id,
        revision: hearing.capture.review.result_revision,
        capture_digest: hearing.capture.capture_digest,
    });
}

fn assert_anchor(
    group: &MeasureDecisionRecordStoredOperation,
    hearing: &PrecautionaryHearingRecordStoredOperation,
) {
    let expected = Some(MeasureDecisionAnchorMaterial::Precautionary(Box::new(
        hearing.capture.clone(),
    )));
    assert_eq!(group.group.review.material.anchor, expected);
    assert_eq!(group.group.decision.anchor, expected);
    measure_decision_group_v2_matches(&RingSha256Hasher, &group.group, &group.record_history)
        .unwrap();
}

#[test]
fn real_review_of_c_is_a_full_g2_anchor_even_for_zero_row_decisions() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let hearing = hearing(&db, &seed, corrected_reference(&seed.corrected.capture));
    let mut command = crate::measure_fixture::no_change(&seed.command);
    attach(&mut command, &hearing);
    let recorded = persist(&db, seed.actor.clone(), command);
    assert!(recorded.group.measures.is_empty());
    assert_anchor(&recorded, &hearing);
    assert_eq!(
        recorded.record_history.records.administrative[0].capture,
        seed.corrected.capture
    );
    assert_eq!(
        recorded.record_history.records.judicial.groups[0].capture,
        seed.judicial.group
    );
    reopened(&db, &seed.actor, &recorded);
}

#[test]
fn real_review_of_m2_preserves_selected_g2_and_c_ancestry_in_later_anchored_effects() {
    let Some(mut db) = Fixture::new() else { return };
    let seed = setup(&mut db);
    let first = persist(&db, seed.actor.clone(), seed.command.clone());
    let selected = reference(&first.group.measures[0]);
    let hearing = hearing(&db, &seed, selected);
    let mut command = effect_command(
        &seed.command,
        vec![MeasureEffect::Confirm { previous: selected }],
    );
    attach(&mut command, &hearing);
    let next = persist(&db, seed.actor.clone(), command);
    assert_eq!(next.group.measures[0].result.revision.get(), 4);
    assert_anchor(&next, &hearing);
    assert_eq!(next.record_history.decisions.len(), 1);
    assert_eq!(next.record_history.decisions[0].capture, first.group);
    assert_eq!(
        next.record_history.records.administrative[0].capture,
        seed.corrected.capture
    );
    reopened(&db, &seed.actor, &next);
}
