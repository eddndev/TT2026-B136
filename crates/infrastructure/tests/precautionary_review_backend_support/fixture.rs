use super::*;

pub struct ReviewSeed {
    pub actor: Principal,
    pub measure_command: MeasureDecisionCommand,
    pub first: MeasureDecisionStoredOperation,
    pub command: PrecautionaryHearingCommand,
}

pub fn reference(capture: &MeasureCapture) -> PrecautionaryMeasureRef {
    PrecautionaryMeasureRef::new(
        capture.result.id,
        capture.result.revision,
        capture.capture_digest,
    )
}

pub fn setup(db: &mut Fixture) -> ReviewSeed {
    let seed = crate::measure_fixture::setup(db);
    let first = crate::measure_fixture::persist(db, seed.actor.clone(), seed.command.clone());
    let values = PrecautionaryHearingValues::new(PrecautionaryHearingValuesInput {
        purpose: PrecautionaryHearingPurpose::Review,
        scheduled_at: HearingTime::new((db.at + Duration::days(2)).replace_nanosecond(0).unwrap())
            .unwrap(),
        modality: HearingModality::InPerson,
        venue: HearingVenue::new("Court for the declared review").unwrap(),
        note: None,
        participants: vec![],
        scheduling_basis: PrecautionaryHearingSchedulingBasis::new(
            HearingNote::new("Declared review appointment").unwrap(),
            HearingSupportRef::new(
                DocumentVersionRef {
                    id: seed.record.id,
                    version: seed.record.version,
                },
                seed.record.digest,
            ),
            HearingNote::new("Page 1").unwrap(),
        ),
        review_targets: first.group.measures.iter().map(reference).collect(),
    })
    .unwrap();
    ReviewSeed {
        actor: seed.actor,
        measure_command: seed.command.clone(),
        first,
        command: PrecautionaryHearingCommand {
            operation_id: PrecautionaryHearingOperationId::new(),
            hearing_id: PrecautionaryHearingId::new(),
            change: PrecautionaryHearingChange::Schedule {
                context: seed.command.context,
                values,
            },
        },
    }
}

pub fn select_targets(
    command: &mut PrecautionaryHearingCommand,
    targets: Vec<PrecautionaryMeasureRef>,
) {
    match &mut command.change {
        PrecautionaryHearingChange::Schedule { values, .. }
        | PrecautionaryHearingChange::Replace { values, .. } => {
            let mut selected = input(values);
            selected.review_targets = targets;
            *values = PrecautionaryHearingValues::new(selected).unwrap();
        }
        PrecautionaryHearingChange::Cancel { .. } => {
            panic!("cancellation retains its exact targets")
        }
    }
}

pub fn replace_targets(
    previous: &PrecautionaryHearingStoredOperation,
    targets: Vec<PrecautionaryMeasureRef>,
) -> PrecautionaryHearingCommand {
    let mut command = crate::hearing_fixture::replacement(previous);
    select_targets(&mut command, targets);
    command
}

pub fn measure_effects(seed: &ReviewSeed, effects: Vec<MeasureEffect>) -> MeasureDecisionCommand {
    let mut command = crate::measure_fixture::fresh(&seed.measure_command);
    command.outcome =
        MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(effects)).unwrap();
    command
}

pub fn confirm_measures(
    db: &Fixture,
    seed: &ReviewSeed,
    targets: &[PrecautionaryMeasureRef],
) -> MeasureDecisionStoredOperation {
    crate::measure_fixture::persist(
        db,
        seed.actor.clone(),
        measure_effects(
            seed,
            targets
                .iter()
                .map(|previous| MeasureEffect::Confirm {
                    previous: *previous,
                })
                .collect(),
        ),
    )
}

pub fn assert_groups(
    history: &MeasureHistoryEvidence,
    expected: &[&MeasureDecisionStoredOperation],
) {
    assert_eq!(history.groups.len(), expected.len());
    for original in expected {
        let actual = history
            .groups
            .iter()
            .find(|entry| entry.origin.operation_id == original.origin.operation_id)
            .unwrap();
        assert_eq!(actual.origin, original.origin);
        assert_eq!(actual.capture, original.group);
    }
}

pub fn same_operation(
    actual: &PrecautionaryHearingStoredOperation,
    expected: &PrecautionaryHearingStoredOperation,
) {
    let mut actual = actual.clone();
    let mut expected = expected.clone();
    actual
        .history
        .measure_history
        .groups
        .sort_by_key(|group| group.origin.operation_id.as_uuid());
    expected
        .history
        .measure_history
        .groups
        .sort_by_key(|group| group.origin.operation_id.as_uuid());
    assert_eq!(actual, expected);
}

pub fn reopened(db: &Fixture, actor: &Principal, expected: &PrecautionaryHearingStoredOperation) {
    let query = reads(db, actor.clone());
    let capture = &expected.capture;
    same_operation(
        &query
            .get(
                "session",
                db.case,
                capture.review.command.hearing_id,
                Some(capture.review.result_revision),
            )
            .unwrap(),
        expected,
    );
    same_operation(
        &query
            .get_operation("session", db.case, capture.review.command.operation_id)
            .unwrap(),
        expected,
    );
    let replay = service_with_format(
        db,
        actor.clone(),
        FormatCheck(Some(Box::new(|| {
            panic!("historical Review replay must skip documentary admission")
        }))),
    );
    same_operation(
        &replay
            .submit(
                "session",
                db.case,
                capture.review.command.clone(),
                confirmation(&capture.review),
            )
            .unwrap(),
        expected,
    );
}
