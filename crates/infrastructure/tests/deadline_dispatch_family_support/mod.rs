use crate::{
    deadline_backend_support as dl, deadline_dispatch_support as dispatch,
    deadline_input_support as inputs, deadline_profile_database_support as profiles,
    hearing_database_support as hearings, hearing_result_database_support as results,
    procedural_fact_backend_support as facts,
};
use application::{
    cases::CaseRepository,
    deadline_dispatch::*,
    deadline_profiles::*,
    deadline_reevaluation::{DependencyFamily, SourceEventReference},
    deadlines::*,
    hearing_results::*,
    procedural_facts::*,
};
use domain::{
    cases::{CaseId, CaseMetadata},
    deadline_triggers::{TriggerField, TriggerRequirement},
    identity::Role,
};
use uuid::Uuid;

pub fn publish(
    db: &dl::Fixture,
    scope: Option<CaseId>,
    field: TriggerField,
) -> DeadlineProfileDetail {
    let mut definition = profiles::input(scope);
    definition.trigger = TriggerRequirement::SourceField(field);
    profiles::persist(
        &profiles::service(db, db.owner, Role::Owner),
        scope.map_or(
            DeadlineProfileCollection::Global,
            DeadlineProfileCollection::ForCase,
        ),
        DeadlineProfileCommand {
            operation_id: DeadlineProfileOperationId::new(),
            profile_id: DeadlineProfileId::new(),
            change: DeadlineProfileChange::Publish {
                definition: DeadlineProfileDefinition::new(definition).unwrap(),
            },
        },
    )
}

pub fn new_case(db: &dl::Fixture) -> CaseId {
    let id = CaseId::new();
    db.store()
        .create_basic(
            db.owner,
            id,
            CaseMetadata::new("Other dispatch case", "DISPATCH-OTHER").unwrap(),
            db.at,
        )
        .unwrap();
    id
}

pub fn notice(db: &dl::Fixture, parent: &FactDetail, id: Uuid) -> FactDetail {
    let parent = facts::resolution_ref(parent);
    facts::persist(
        &facts::service(db, db.owner, Role::Owner),
        db.case,
        ProceduralFactCommand::Notification(
            NotificationCommand::new(
                FactOperationId::new(),
                NotificationId::from_uuid(id),
                parent.id,
                FactChange::record(inputs::notification_at(parent, "2026-01-09")),
            )
            .unwrap(),
        ),
    )
}

pub fn resolution(db: &dl::Fixture, id: Uuid) -> FactDetail {
    facts::persist(
        &facts::service(db, db.owner, Role::Owner),
        db.case,
        ProceduralFactCommand::Resolution(ResolutionCommand::new(
            FactOperationId::new(),
            ResolutionId::from_uuid(id),
            FactChange::record(inputs::resolution_at(
                &facts::values("Typed resolution"),
                "2026-01-06",
            )),
        )),
    )
}

pub fn result_in_current_case(db: &dl::Fixture) -> HearingResultDetail {
    let hearing = hearings::persist(
        &hearings::service(db, db.owner, Role::Owner),
        db.case,
        hearings::schedule(),
    );
    let mut command = results::record(hearing.snapshot.id);
    let HearingResultChange::Record { values, .. } = &mut command.change else {
        unreachable!()
    };
    *values = inputs::hearing_values(true, true);
    results::persist(
        &results::service(db, db.owner, Role::Owner),
        db.case,
        command,
    )
}

pub fn persist(db: &dl::Fixture, command: DeadlineCommand) -> DeadlineDetail {
    dl::persist(&dl::service(db, db.owner, Role::Owner), db.case, command)
}

pub fn assert_event(
    db: &mut dl::Fixture,
    store: &dyn DeadlineDispatchStore,
    operation: Uuid,
    expected_ids: &[u128],
) -> SourceEventReference {
    let row = db
        .admin
        .query_one(
            "SELECT sequence,source_kind,source_id,revision,case_id,hearing_id
         FROM deadline_source_events WHERE operation_id=$1",
            &[&operation],
        )
        .unwrap();
    let family = match row.get::<_, &str>(1) {
        "resolution" => DependencyFamily::Resolution,
        "notification" => DependencyFamily::Notification,
        "hearing_result" => DependencyFamily::HearingResult,
        "calendar" => DependencyFamily::Calendar,
        "profile" => DependencyFamily::Profile,
        other => panic!("unexpected persisted family: {other}"),
    };
    let expected = SourceEventReference {
        sequence: u64::try_from(row.get::<_, i64>(0)).unwrap(),
        family,
        source_id: row.get(2),
        revision: u32::try_from(row.get::<_, i64>(3)).unwrap(),
        case_id: row.get::<_, Option<Uuid>>(4).map(CaseId::from_uuid),
        hearing_id: row.get(5),
        operation_id: operation,
    };
    let batch = dispatch::dispatch(store, DeadlineDispatchStream::Events, 100);
    assert_eq!(batch.event, Some(expected));
    assert_eq!(batch.selected as usize, expected_ids.len());
    assert_eq!(batch.inserted, batch.selected);
    assert!(batch.completed_scan);
    assert_eq!(
        batch.progress.event.completed_sequence,
        Some(expected.sequence)
    );
    assert!(batch.progress.event.active_sequence.is_none());
    assert!(batch.progress.event.after_deadline_id.is_none());
    assert_eq!(
        dispatch::event_jobs(db, expected.sequence),
        expected_ids
            .iter()
            .copied()
            .map(dispatch::id)
            .collect::<Vec<_>>()
    );
    let mismatched_scope: bool = db
        .admin
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM deadline_reevaluation_jobs j
         JOIN case_deadlines d ON d.id=j.deadline_id
         WHERE j.event_sequence=$1 AND j.case_id<>d.case_id)",
            &[&i64::try_from(expected.sequence).unwrap()],
        )
        .unwrap()
        .get(0);
    assert!(!mismatched_scope);
    expected
}
