use crate::{
    deadline_backend_support as dl, deadline_profile_database_support as profiles,
    deadline_tracked_backend_support as tracked, procedural_fact_backend_support as facts,
};
use application::{deadline_profiles::*, deadlines::*, procedural_facts::*};
use domain::{
    deadline_triggers::{TriggerField, TriggerRequirement, TriggerSourceRef},
    identity::Role,
    procedural_time::DeclaredProceduralTime,
};
use infrastructure::RingSha256Hasher;

pub fn setup(db: &dl::Fixture) -> (DeadlineCommand, FactDetail, FactDetail) {
    let parent = dl::source(db);
    let reference = facts::resolution_ref(&parent);
    let original = facts::notification_values(reference, "Dated notification");
    let values = NotificationValues::new(NotificationValuesInput {
        resolution: reference,
        character: original.character().clone(),
        medium: original.medium().clone(),
        context: original.context().clone(),
        outcome: original.outcome().clone(),
        subtype: None,
        practiced_at: DeclaredProceduralTime::date("2026-01-09".parse().unwrap(), None).unwrap(),
        received_at: None,
        stated_effect: None,
        intended_recipient: original.intended_recipient().clone(),
        actual_receiver: original.actual_receiver().clone(),
        representation: original.representation().clone(),
        summary: original.summary().clone(),
        provenance: original.provenance().clone(),
    })
    .unwrap();
    let notification = facts::persist(
        &facts::service(db, db.owner, Role::Owner),
        db.case,
        ProceduralFactCommand::Notification(
            NotificationCommand::new(
                FactOperationId::new(),
                NotificationId::new(),
                reference.id,
                FactChange::record(values),
            )
            .unwrap(),
        ),
    );
    let mut definition = profiles::input(Some(db.case));
    definition.trigger = TriggerRequirement::SourceField(TriggerField::NotificationPracticedAt);
    let profile = profiles::persist(
        &profiles::service(db, db.owner, Role::Owner),
        DeadlineProfileCollection::ForCase(db.case),
        DeadlineProfileCommand {
            operation_id: DeadlineProfileOperationId::new(),
            profile_id: DeadlineProfileId::new(),
            change: DeadlineProfileChange::Publish {
                definition: DeadlineProfileDefinition::new(definition).unwrap(),
            },
        },
    );
    let mut command = dl::command(db, &profile, &parent);
    let FactTarget::Notification { id, .. } = notification.snapshot.target() else {
        panic!("notification expected")
    };
    dl::definition_mut(&mut command).input.selection.source =
        FactDeclaration::Known(TriggerSourceRef::Notification {
            id,
            revision: notification.snapshot.metadata().revision,
            resolution: reference,
        });
    (command, parent, notification)
}

pub fn prepare(
    db: &dl::Fixture,
    store: &dyn DeadlineStore,
    command: &DeadlineCommand,
    parent_head: &FactDetail,
) -> PreparedDeadlineChange {
    prepare_tracked_deadline_change(
        &RingSha256Hasher,
        DeadlineActorSnapshot::User {
            id: db.owner,
            email: "owner@example.test".into(),
        },
        db.case,
        command.clone(),
        store.prepare(db.owner, db.case, command).unwrap(),
        Some(tracked::policies()),
        Some(parent_head),
    )
    .expect("current notification and explicit parent head must qualify")
}

pub fn advance_parent(db: &dl::Fixture, parent: &FactDetail) -> FactDetail {
    facts::persist(
        &facts::service(db, db.owner, Role::Owner),
        db.case,
        facts::correct(parent),
    )
}
