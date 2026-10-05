use super::*;
use application::cases::CaseRevisionExpectation;
use application::participants::{
    DirectoryStatus, ParticipantId, ParticipantStore, ParticipantValues,
};
use application::typed_participants::ParticipantRevisionSnapshot;
use infrastructure::PostgresParticipantStore;

#[test]
fn retained_manual_participant_keeps_exact_full_source_after_edit_and_archive() {
    let Some(mut db) = Fixture::new() else { return };
    let (actor, mut command) = setup(&mut db);
    let participants =
        PostgresParticipantStore::open(&db.runtime_url, Arc::new(RingSha256Hasher)).unwrap();
    let source = participants
        .create(
            db.owner,
            db.case,
            ParticipantId::new(),
            ParticipantValues::new(
                "Original witness",
                "Witness",
                None,
                Some("Declared appearance"),
                DirectoryStatus::Active,
            )
            .unwrap(),
            db.at,
        )
        .unwrap();
    let PrecautionaryHearingChange::Schedule { values, .. } = &mut command.change else {
        unreachable!()
    };
    let mut changed = input(values);
    changed
        .participants
        .push(HearingParticipantRef::new(source.id, source.revision));
    *values = PrecautionaryHearingValues::new(changed).unwrap();
    let first = persist(&db, actor.clone(), command.clone());
    let edited = participants
        .replace(
            db.owner,
            db.case,
            source.id,
            source.revision,
            ParticipantValues::new(
                "Edited witness",
                "Witness",
                None,
                None,
                DirectoryStatus::Active,
            )
            .unwrap(),
            db.at,
        )
        .unwrap();
    participants
        .change_status(
            db.owner,
            db.case,
            source.id,
            edited.revision,
            DirectoryStatus::Archived,
            db.at,
        )
        .unwrap();
    let second = persist(&db, actor.clone(), replacement(&first));
    assert_eq!(second.capture.review.sources, first.capture.review.sources);
    assert_eq!(
        second.capture.review.participants,
        first.capture.review.participants
    );
    let selected = &second.capture.review.sources.participants[0];
    assert_eq!(
        selected.revision,
        ParticipantRevisionSnapshot::Manual(Box::new(source))
    );
    assert_eq!(
        reads(&db, actor.clone())
            .get(
                "session",
                db.case,
                command.hearing_id,
                Some(first.capture.review.result_revision)
            )
            .unwrap(),
        first
    );
    command.hearing_id = PrecautionaryHearingId::new();
    command.operation_id = PrecautionaryHearingOperationId::new();
    let before = snapshot(&mut db);
    assert!(service(&db, actor)
        .prepare("session", db.case, command)
        .is_err());
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn cancellation_captures_new_observed_administration_but_retains_original_stage_and_schedule() {
    let Some(mut db) = Fixture::new() else { return };
    let (actor, command) = setup(&mut db);
    let first = persist(&db, actor.clone(), command);
    let original = first.capture.review.observed_context.clone();
    let values = creation("UPDATED-NUC").into_values().editable().clone();
    let updated = db
        .store()
        .replace_administration(
            db.owner,
            db.case,
            CaseRevisionExpectation::Revision(original.material().administration.revision),
            values,
            db.at,
        )
        .unwrap();
    let cancelled = persist(&db, actor.clone(), cancellation(&first));
    assert_eq!(
        cancelled.capture.review.scheduling_context,
        first.capture.review.scheduling_context
    );
    assert_eq!(
        cancelled
            .capture
            .review
            .observed_context
            .material()
            .administration,
        *updated.administration.snapshot().unwrap()
    );
    assert_eq!(
        cancelled
            .capture
            .review
            .observed_context
            .material()
            .stage_administration,
        original.material().stage_administration
    );
    assert_eq!(
        cancelled.capture.review.observed_context.material().stage,
        original.material().stage
    );
    assert_eq!(
        reads(&db, actor)
            .get(
                "session",
                db.case,
                first.capture.review.command.hearing_id,
                None
            )
            .unwrap(),
        cancelled
    );
}

#[test]
fn the_store_reloads_complete_principal_before_operation_lookup() {
    use application::documents::StageSupportReadLimits;
    let Some(mut db) = Fixture::new() else { return };
    let (actor, command) = setup(&mut db);
    let first = persist(&db, actor.clone(), command.clone());
    let storage = store(&db);
    for mutation in 0..2 {
        let mut stale = actor.clone();
        if mutation == 0 {
            stale.email = "stale@example.test".into();
        } else {
            stale.role = Role::Litigator;
        }
        let before = snapshot(&mut db);
        assert!(storage
            .prepare(
                &stale,
                db.case,
                &command,
                &StageSupportReadLimits::standard()
            )
            .is_err());
        assert!(storage
            .get_operation(&stale, db.case, command.operation_id)
            .is_err());
        assert_eq!(snapshot(&mut db), before);
    }
    assert_eq!(
        storage
            .get_operation(&actor, db.case, command.operation_id)
            .unwrap(),
        first
    );
}
