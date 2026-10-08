use super::*;

pub struct AnchorSeed {
    pub seed: Seed,
    pub hearing: PrecautionaryHearingStoredOperation,
    pub command: MeasureDecisionCommand,
}

pub fn reference(capture: &MeasureCapture) -> PrecautionaryMeasureRef {
    PrecautionaryMeasureRef::new(
        capture.result.id,
        capture.result.revision,
        capture.capture_digest,
    )
}

pub fn anchor(hearing: &PrecautionaryHearingStoredOperation) -> MeasureDecisionAnchorRef {
    let capture = &hearing.capture;
    MeasureDecisionAnchorRef::Precautionary {
        hearing_id: capture.review.command.hearing_id,
        revision: capture.review.result_revision,
        capture_digest: capture.capture_digest,
    }
}

pub fn expected_anchor(
    hearing: &PrecautionaryHearingStoredOperation,
) -> Option<MeasureDecisionAnchorMaterial> {
    Some(MeasureDecisionAnchorMaterial::Precautionary(Box::new(
        hearing.capture.clone(),
    )))
}

pub fn hearing_command(
    db: &Fixture,
    seed: &Seed,
    targets: Vec<PrecautionaryMeasureRef>,
) -> PrecautionaryHearingCommand {
    let purpose = if targets.is_empty() {
        PrecautionaryHearingPurpose::Imposition
    } else {
        PrecautionaryHearingPurpose::Review
    };
    let values = PrecautionaryHearingValues::new(PrecautionaryHearingValuesInput {
        purpose,
        scheduled_at: HearingTime::new((db.at + Duration::days(2)).replace_nanosecond(0).unwrap())
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
        review_targets: targets,
    })
    .unwrap();
    PrecautionaryHearingCommand {
        operation_id: PrecautionaryHearingOperationId::new(),
        hearing_id: PrecautionaryHearingId::new(),
        change: PrecautionaryHearingChange::Schedule {
            context: seed.command.context,
            values,
        },
    }
}

pub fn setup(db: &mut Fixture) -> AnchorSeed {
    let seed = crate::measure_fixture::setup(db);
    let hearing =
        crate::hearing_fixture::persist(db, seed.actor.clone(), hearing_command(db, &seed, vec![]));
    let mut command = seed.command.clone();
    command.anchor = Some(anchor(&hearing));
    AnchorSeed {
        seed,
        hearing,
        command,
    }
}

pub fn assert_anchor(
    operation: &MeasureDecisionStoredOperation,
    hearing: &PrecautionaryHearingStoredOperation,
) {
    assert_eq!(operation.group.review.command.anchor, Some(anchor(hearing)));
    assert_eq!(
        operation.group.review.material.anchor,
        expected_anchor(hearing)
    );
    assert_eq!(operation.group.decision.anchor, expected_anchor(hearing));
    measure_decision_group_with_history_matches(
        &RingSha256Hasher,
        &operation.group,
        &operation.measure_history,
    )
    .unwrap();
}

pub fn effects(
    seed: &Seed,
    hearing: &PrecautionaryHearingStoredOperation,
    changes: Vec<MeasureEffect>,
) -> MeasureDecisionCommand {
    let mut command = fresh(&seed.command);
    command.anchor = Some(anchor(hearing));
    command.outcome =
        MeasureDecisionOutcome::new(MeasureDecisionOutcomeInput::Changes(changes)).unwrap();
    command
}

pub fn replace_targets(
    previous: &PrecautionaryHearingStoredOperation,
    targets: Vec<PrecautionaryMeasureRef>,
) -> PrecautionaryHearingCommand {
    let mut command = crate::hearing_fixture::replacement(previous);
    let PrecautionaryHearingChange::Replace { values, .. } = &mut command.change else {
        unreachable!()
    };
    let mut selected = crate::hearing_fixture::input(values);
    selected.purpose = if targets.is_empty() {
        PrecautionaryHearingPurpose::Imposition
    } else {
        PrecautionaryHearingPurpose::Review
    };
    selected.review_targets = targets;
    *values = PrecautionaryHearingValues::new(selected).unwrap();
    command
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

fn same_operation(
    actual: &MeasureDecisionStoredOperation,
    expected: &MeasureDecisionStoredOperation,
) {
    let mut actual = actual.clone();
    let mut expected = expected.clone();
    actual
        .measure_history
        .groups
        .sort_by_key(|g| g.origin.operation_id.as_uuid());
    expected
        .measure_history
        .groups
        .sort_by_key(|g| g.origin.operation_id.as_uuid());
    assert_eq!(actual, expected);
}

pub fn reopened(db: &Fixture, actor: &Principal, expected: &MeasureDecisionStoredOperation) {
    let bytes = measure_decision_group_bytes(&expected.group).unwrap();
    let query = reads(db, actor.clone());
    let loaded = query
        .get("session", db.case, expected.origin.decision_id)
        .unwrap();
    same_operation(&loaded, expected);
    assert_eq!(measure_decision_group_bytes(&loaded.group).unwrap(), bytes);
    same_operation(
        &query
            .get_operation("session", db.case, expected.origin.operation_id)
            .unwrap(),
        expected,
    );
    let replay = service_with_format(
        db,
        actor.clone(),
        FormatCheck(Some(Box::new(|| {
            panic!("replay must retain original support admission")
        }))),
    );
    assert_eq!(
        replay
            .prepare("session", db.case, expected.group.review.command.clone())
            .unwrap(),
        expected.group.review
    );
    same_operation(
        &replay
            .submit(
                "session",
                db.case,
                expected.group.review.command.clone(),
                confirmation(&expected.group.review),
            )
            .unwrap(),
        expected,
    );
}

pub fn reopened_hearing(
    db: &Fixture,
    actor: &Principal,
    expected: &PrecautionaryHearingStoredOperation,
) {
    let query = crate::hearing_fixture::reads(db, actor.clone());
    let loaded = query
        .get(
            "session",
            db.case,
            expected.capture.review.command.hearing_id,
            Some(expected.capture.review.result_revision),
        )
        .unwrap();
    assert_eq!(loaded.capture, expected.capture);
    assert_eq!(loaded.history.origin, expected.history.origin);
    assert_eq!(loaded.history.captures, expected.history.captures);
    let mut actual = loaded.history.measure_history.groups;
    let mut wanted = expected.history.measure_history.groups.clone();
    actual.sort_by_key(|g| g.origin.operation_id.as_uuid());
    wanted.sort_by_key(|g| g.origin.operation_id.as_uuid());
    assert_eq!(actual, wanted);
}
