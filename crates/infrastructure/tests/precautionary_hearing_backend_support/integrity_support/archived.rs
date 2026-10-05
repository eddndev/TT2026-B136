use super::*;

#[test]
fn coherent_historical_schedule_cannot_introduce_an_archived_participant_revision() {
    let Some(mut db) = Fixture::new() else { return };
    let (actor, command) = setup(&mut db);
    let participants =
        PostgresParticipantStore::open(&db.runtime_url, Arc::new(RingSha256Hasher)).unwrap();
    let active = participants
        .create(
            db.owner,
            db.case,
            ParticipantId::new(),
            ParticipantValues::new("Witness", "Witness", None, None, DirectoryStatus::Active)
                .unwrap(),
            db.at,
        )
        .unwrap();
    let archived = participants
        .change_status(
            db.owner,
            db.case,
            active.id,
            active.revision,
            DirectoryStatus::Archived,
            db.at,
        )
        .unwrap();
    let original = persist(&db, actor.clone(), command.clone());
    let storage = store(&db);
    let mut selected = input(&original.capture.review.resolved_values);
    selected.participants = vec![HearingParticipantRef::new(
        archived.id(),
        archived.revision_number(),
    )];
    let values = PrecautionaryHearingValues::new(selected).unwrap();
    let mut forged_command = command.clone();
    forged_command.change = PrecautionaryHearingChange::Schedule {
        context: expectation(&original.capture.review.observed_context),
        values,
    };
    let mut forged_sources = original.capture.review.sources.clone();
    forged_sources.participants = vec![archived];
    let mut capture = prepare_precautionary_hearing_capture(
        &RingSha256Hasher,
        &actor,
        db.case,
        forged_command,
        original.capture.review.observed_context.clone(),
        forged_sources,
        None,
    )
    .unwrap()
    .into_capture(&RingSha256Hasher, db.at)
    .unwrap();
    // Historical fact resolution proves exact bytes, not fresh selection authority.
    precautionary_hearing_receipt_matches(&RingSha256Hasher, &capture).unwrap();
    rewrite_capture(&mut db, &mut capture);
    rejected_reads_and_reopen(&mut db, &actor, &command, &storage);
}
