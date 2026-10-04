use super::*;

#[test]
fn participant_head_is_rechecked_but_historical_replay_keeps_its_capture() {
    use application::participants::{
        DirectoryStatus, ParticipantId, ParticipantStore, ParticipantValues,
    };
    use domain::{hearings::HearingParticipantRef, resource_hearings::*};
    let Some(mut db) = Fixture::new() else { return };
    let (_, mut command) = setup(&mut db);
    let participants = infrastructure::PostgresParticipantStore::open(
        &db.runtime_url,
        std::sync::Arc::new(infrastructure::RingSha256Hasher),
    )
    .unwrap();
    let person = participants
        .create(
            db.owner,
            db.case,
            ParticipantId::new(),
            ParticipantValues::new("Witness A", "Witness", None, None, DirectoryStatus::Active)
                .unwrap(),
            db.at,
        )
        .unwrap();
    let v = &command.values;
    command.values = ResourceHearingValues::new(ResourceHearingValuesInput {
        kind: v.kind(),
        scheduled_at: v.scheduled_at(),
        modality: v.modality(),
        venue: v.venue().clone(),
        note: v.note().cloned(),
        participants: vec![HearingParticipantRef::new(person.id, person.revision)],
        scheduling_basis: v.scheduling_basis().clone(),
    })
    .unwrap();
    let original = submit(&db, command.clone());
    let backend = store(&db);
    let mut other = command.clone();
    other.hearing_id = ResourceHearingId::new();
    other.operation_id = ResourceHearingOperationId::new();
    other.association_id = ResourceActivityId::new();
    let ResourceHearingPreparation::Ready(material) = backend
        .prepare(db.owner, db.case, other.resource.id, &other)
        .unwrap()
    else {
        panic!("new operation")
    };
    let actor = application::identity::Principal {
        id: db.owner,
        email: "owner@example.test".into(),
        role: Role::Owner,
    };
    let prepared = prepare_resource_hearing_change(
        std::sync::Arc::new(infrastructure::RingSha256Hasher),
        &actor,
        db.case,
        other.resource.id,
        other.clone(),
        *material,
    )
    .unwrap();
    participants
        .replace(
            db.owner,
            db.case,
            person.id,
            person.revision,
            ParticipantValues::new("Witness B", "Witness", None, None, DirectoryStatus::Active)
                .unwrap(),
            db.at,
        )
        .unwrap();
    let before = atomic_rows(&mut db);
    assert!(matches!(
        backend.commit(db.owner, db.case, other.resource.id, prepared),
        Err(ApplicationError::ResourceActivity(
            ResourceActivityError::SourceMismatch
        ))
    ));
    assert_eq!(atomic_rows(&mut db), before);
    assert_eq!(
        service(&db, db.owner, Role::Owner)
            .submit(
                "session",
                db.case,
                command.resource.id,
                command,
                original.origin.submission_digest
            )
            .unwrap(),
        original
    );
}
