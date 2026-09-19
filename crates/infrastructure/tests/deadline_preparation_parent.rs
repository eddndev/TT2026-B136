mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
#[allow(dead_code)]
mod deadline_dispatch_family_support;
#[allow(dead_code)]
mod deadline_dispatch_support;
mod deadline_input_support;
mod deadline_profile_database_support;
mod deadline_tracked_backend_support;
mod deadline_tracked_notification_support;
mod hearing_database_support;
mod hearing_result_database_support;
mod judicial_calendar_database_support;
mod procedural_fact_backend_support;

use application::{
    deadline_inputs::DeadlineSourceDetail,
    deadlines::*,
    procedural_facts::{fact_receipt_matches, FactDetail, FactHearingRef, ProceduralFactSnapshot},
};
use deadline_backend_support as dl;
use deadline_dispatch_family_support as family;
use deadline_tracked_notification_support as notification;
use domain::{
    deadline_triggers::{TriggerField, TriggerSourceRef},
    procedural_facts::FactDeclaration,
};
use infrastructure::RingSha256Hasher;
use procedural_fact_backend_support as facts;

fn assert_notification_parent(
    preparation: &DeadlinePreparation,
    command: &DeadlineCommand,
    parent: &FactDetail,
    parent_head: &FactDetail,
    notice: &FactDetail,
) {
    assert_eq!(preparation.case_id, notice.snapshot.case_id());
    assert_eq!(preparation.deadline_id, command.deadline_id);
    let resolved = preparation.resolved.as_ref().unwrap();
    assert_eq!(resolved.material.case_id, preparation.case_id);
    assert_eq!(resolved.material.administration, preparation.administration);
    let observed = resolved.notification_parent_head.as_ref().unwrap();
    assert_eq!(observed, parent_head);
    assert_eq!(observed.snapshot.case_id(), preparation.case_id);
    assert_eq!(
        facts::resolution_ref(observed).id,
        facts::resolution_ref(parent).id
    );
    assert_eq!(facts::resolution_ref(parent).revision.get(), 1);
    assert_eq!(facts::resolution_ref(observed).revision.get(), 2);
    fact_receipt_matches(&RingSha256Hasher, observed).unwrap();

    let Some(DeadlineSourceDetail::Fact(selected)) = &resolved.material.source else {
        panic!("exact notification material expected")
    };
    assert_eq!(selected.as_ref(), notice);
    assert_eq!(resolved.material.source_head, resolved.material.source);
    fact_receipt_matches(&RingSha256Hasher, selected).unwrap();
    let ProceduralFactSnapshot::Notification(snapshot) = &selected.snapshot else {
        panic!("notification snapshot expected")
    };
    assert_eq!(snapshot.metadata.revision.get(), 1);
    assert_eq!(snapshot.values.resolution(), facts::resolution_ref(parent));
    assert_eq!(
        selected
            .sources
            .resolved
            .resolution
            .as_ref()
            .unwrap()
            .reference,
        facts::resolution_ref(parent)
    );
    assert_eq!(
        selected
            .sources
            .views
            .resolution
            .as_ref()
            .unwrap()
            .reference,
        facts::resolution_ref(parent)
    );
}

#[test]
fn register_and_correct_prepare_the_current_parent_without_replacing_notification_history() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let (register, parent, notice) = notification::setup(&db);
    let parent_head = notification::advance_parent(&db, &parent);
    let store = dl::store(&db);
    let before = dl::snapshot(&mut db);
    let preparation = store.prepare(db.owner, db.case, &register).unwrap();
    assert!(preparation.base.is_none());
    assert_notification_parent(&preparation, &register, &parent, &parent_head, &notice);
    assert_eq!(dl::snapshot(&mut db), before);

    let prepared = notification::prepare(&db, store.as_ref(), &register, &parent_head);
    let first = store.commit(db.owner, prepared).unwrap();
    let correct = dl::correct(&first);
    let before = dl::snapshot(&mut db);
    let preparation = store.prepare(db.owner, db.case, &correct).unwrap();
    assert_eq!(preparation.base.as_ref(), Some(&first));
    assert_notification_parent(&preparation, &correct, &parent, &parent_head, &notice);
    assert_eq!(dl::snapshot(&mut db), before);
}

#[test]
fn other_source_selections_never_inherit_a_previous_notifications_parent() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let hearing = deadline_input_support::hearing_result(&mut db, true);
    let (original, parent, _) = notification::setup(&db);
    let store = dl::store(&db);
    let prepared = notification::prepare(&db, store.as_ref(), &original, &parent);
    let base = store.commit(db.owner, prepared).unwrap();
    let resolution_profile = dl::profile(&db);
    let hearing_profile =
        family::publish(&db, Some(db.case), TriggerField::HearingSessionEventTime);
    let choices = [
        (
            resolution_profile.clone(),
            FactDeclaration::Known(TriggerSourceRef::Resolution(facts::resolution_ref(&parent))),
            Some(DeadlineSourceDetail::Fact(Box::new(parent.clone()))),
        ),
        (
            hearing_profile,
            FactDeclaration::Known(TriggerSourceRef::HearingResult(FactHearingRef {
                hearing_id: hearing.snapshot.hearing_id,
                result_id: hearing.snapshot.id,
                revision: hearing.snapshot.revision,
                agreement_id: None,
            })),
            Some(DeadlineSourceDetail::HearingResult(Box::new(hearing))),
        ),
        (
            resolution_profile,
            FactDeclaration::Unknown(dl::text("The source has not been identified")),
            None,
        ),
    ];
    let before = dl::snapshot(&mut db);
    for (profile, source, expected_material) in choices {
        let mut register = dl::command(&db, &profile, &parent);
        dl::definition_mut(&mut register).input.selection.source = source;
        let mut correct = dl::correct(&base);
        *dl::definition_mut(&mut correct) = dl::definition_mut(&mut register).clone();
        for command in [register, correct] {
            let preparation = store.prepare(db.owner, db.case, &command).unwrap();
            if command.action() == DeadlineAction::Correct {
                assert_eq!(preparation.base.as_ref(), Some(&base));
            }
            assert_eq!(preparation.case_id, db.case);
            assert_eq!(preparation.deadline_id, command.deadline_id);
            let resolved = preparation.resolved.unwrap();
            assert!(resolved.notification_parent_head.is_none());
            assert_eq!(resolved.material.case_id, db.case);
            assert_eq!(resolved.material.source, expected_material);
            assert_eq!(resolved.material.source_head, expected_material);
        }
    }
    assert_eq!(dl::snapshot(&mut db), before);
}
