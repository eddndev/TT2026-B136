use super::*;

pub fn hearing_command(
    db: &Fixture,
    seed: &Seed,
    targets: Vec<PrecautionaryMeasureRef>,
) -> PrecautionaryHearingCommand {
    PrecautionaryHearingCommand {
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
                venue: HearingVenue::new("Court for the declared review").unwrap(),
                note: None,
                participants: vec![],
                scheduling_basis: PrecautionaryHearingSchedulingBasis::new(
                    note("Declared review appointment"),
                    seed.command.values.support(),
                    note("Page 1"),
                ),
                review_targets: targets,
            })
            .unwrap(),
        },
    }
}

pub fn review_hearing(
    db: &Fixture,
    seed: &Seed,
    targets: Vec<PrecautionaryMeasureRef>,
) -> PrecautionaryHearingStoredOperation {
    crate::hearing_fixture::persist(db, seed.actor.clone(), hearing_command(db, seed, targets))
}

pub fn replace_with_imposition(
    previous: &PrecautionaryHearingStoredOperation,
) -> PrecautionaryHearingCommand {
    let mut command = crate::hearing_fixture::replacement(previous);
    let PrecautionaryHearingChange::Replace { values, .. } = &mut command.change else {
        unreachable!()
    };
    let mut selected = crate::hearing_fixture::input(values);
    selected.purpose = PrecautionaryHearingPurpose::Imposition;
    selected.review_targets.clear();
    *values = PrecautionaryHearingValues::new(selected).unwrap();
    command
}

pub fn anchor(hearing: &PrecautionaryHearingStoredOperation) -> MeasureDecisionAnchorRef {
    MeasureDecisionAnchorRef::Precautionary {
        hearing_id: hearing.capture.review.command.hearing_id,
        revision: hearing.capture.review.result_revision,
        capture_digest: hearing.capture.capture_digest,
    }
}

pub fn inventory_snapshot(client: &mut postgres::Client) -> serde_json::Value {
    client.query_one(
        "SELECT jsonb_build_object(
          'operations',(SELECT jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text) FROM case_measure_operations r),
          'decisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text) FROM case_measure_decisions r),
          'administrations',(SELECT jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text) FROM case_measure_administrations r),
          'roots',(SELECT jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text) FROM case_measures r),
          'revisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text) FROM case_measure_revisions r),
          'hearings',(SELECT jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text) FROM case_precautionary_hearings r),
          'hearing_revisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text) FROM case_precautionary_hearing_revisions r),
          'audit',(SELECT jsonb_agg(to_jsonb(r) ORDER BY sequence) FROM audit_events r))",
        &[],
    ).unwrap().get(0)
}

pub fn known_dependants(
    db: &mut Fixture,
    workflow: &MeasureAdministrativeService,
    command: &MeasureAdministrativeCommand,
    original: &MeasureAdministrativeReview,
) {
    let before = inventory_snapshot(&mut db.admin);
    for result in [
        workflow
            .prepare("session", db.case, command.clone())
            .map(|_| ()),
        workflow
            .submit("session", db.case, command.clone(), confirmation(original))
            .map(|_| ()),
    ] {
        assert!(matches!(
            result,
            Err(ApplicationError::MeasureAdministrative(
                MeasureAdministrativeError::KnownDependants
            ))
        ));
    }
    assert_eq!(inventory_snapshot(&mut db.admin), before);
}
