mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;
mod hearing_revalidation_support;
mod typed_participant_service_support;

use application::{
    case_stages::{CaseStageWorkflow, DeclaredStageTime, StageTransition},
    cases::*,
    hearings::*,
    participants::*,
    ApplicationError,
};
use domain::identity::Role;
use hearing_database_support::*;
use hearing_revalidation_support::*;
use std::sync::Arc;

fn participant_store(db: &Fixture) -> infrastructure::PostgresParticipantStore {
    infrastructure::PostgresParticipantStore::open(
        &db.runtime_url,
        Arc::new(infrastructure::RingSha256Hasher),
    )
    .unwrap()
}
fn participant_values(name: &str) -> ParticipantValues {
    ParticipantValues::new(name, "Witness", None, None, DirectoryStatus::Active).unwrap()
}
fn with_participant(command: &mut HearingCommand, participant: &ParticipantSnapshot) {
    let HearingChange::Schedule { values, .. } = &mut command.change else {
        unreachable!()
    };
    *values = HearingValues::new(HearingValuesInput {
        kind: values.kind(),
        scheduled_at: values.scheduled_at(),
        modality: values.modality(),
        venue: values.venue().clone(),
        note: values.note().cloned(),
        participants: vec![HearingParticipantRef::new(
            participant.id,
            participant.revision,
        )],
        conviction_basis: None,
    })
    .unwrap();
}

#[test]
fn retained_revisions_survive_edit_and_archive_but_new_selection_requires_current_active() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&mut db);
    let participants = participant_store(&db);
    let first = participants
        .create(
            db.owner,
            db.case,
            ParticipantId::new(),
            participant_values("Original witness"),
            db.at,
        )
        .unwrap();
    let workflow = service(&db, db.owner, Role::Owner);
    let mut command = schedule();
    with_participant(&mut command, &first);
    let recorded = persist(&workflow, db.case, command);
    let second = participants
        .replace(
            db.owner,
            db.case,
            first.id,
            first.revision,
            participant_values("Current witness"),
            db.at,
        )
        .unwrap();
    participants
        .change_status(
            db.owner,
            db.case,
            first.id,
            second.revision,
            DirectoryStatus::Archived,
            db.at,
        )
        .unwrap();
    let replaced = persist(&workflow, db.case, replacement(&recorded));
    assert_eq!(replaced.participants, recorded.participants);
    assert_eq!(
        replaced.participants[0].overview.display_name,
        "Original witness"
    );
    assert_eq!(replaced.participants[0].overview.revision, first.revision);
    assert_eq!(
        workflow
            .get(
                "session",
                db.case,
                recorded.snapshot.id,
                Some(recorded.snapshot.revision)
            )
            .unwrap(),
        recorded
    );
    for stale in [first, second] {
        let mut another = schedule();
        with_participant(&mut another, &stale);
        let before = snapshot(&mut db.admin);
        assert!(matches!(
            workflow.prepare("session", db.case, another),
            Err(ApplicationError::Hearing(HearingError::ParticipantChanged))
        ));
        assert_eq!(snapshot(&mut db.admin), before);
    }
}

#[test]
fn a_new_participant_edited_after_preparation_cannot_silently_enter_the_hearing() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&mut db);
    let participants = participant_store(&db);
    let first = participants
        .create(
            db.owner,
            db.case,
            ParticipantId::new(),
            participant_values("Original"),
            db.at,
        )
        .unwrap();
    let mut command = schedule();
    with_participant(&mut command, &first);
    let draft = service(&db, db.owner, Role::Owner)
        .prepare("session", db.case, command.clone())
        .unwrap();
    let (actor, case, at) = (db.owner, db.case, db.at);
    let (workflow, captured) = watched(&db, actor, Role::Owner, move |_| {
        participants
            .replace(
                actor,
                case,
                first.id,
                first.revision,
                participant_values("Edited during preparation"),
                at,
            )
            .unwrap();
    });
    let result = workflow.submit("session", case, command, draft.submission_digest);
    assert!(
        matches!(
            result,
            Err(ApplicationError::Hearing(HearingError::ParticipantChanged))
        ),
        "{result:?}"
    );
    unchanged(&mut db, captured);
}

#[test]
fn stage_changes_block_scheduling_but_allow_cancellation_with_original_scheduling_context() {
    let Some(mut db) = Fixture::new() else { return };
    complete(&mut db);
    let workflow = service(&db, db.owner, Role::Owner);
    let recorded = persist(&workflow, db.case, schedule());
    let another = schedule();
    let draft = workflow
        .prepare("session", db.case, another.clone())
        .unwrap();
    let record = case_stage_database_support::upload(&db, db.case, "accusation.pdf");
    let stage_workflow = case_stage_database_support::service(
        &db,
        db.owner,
        Role::Owner,
        case_stage_database_support::FormatCheck(None),
    );
    let (actor, case, at) = (db.owner, db.case, db.at);
    let (changing, captured) = watched(&db, actor, Role::Owner, move |_| {
        stage_workflow
            .transition(
                "session",
                case,
                domain::case_administration::CaseStageRevision::FIRST,
                StageTransition::to_intermediate(
                    DeclaredStageTime::instant(at).unwrap(),
                    case_stage_database_support::reference(&record),
                    None,
                ),
            )
            .unwrap();
    });
    let result = changing.submit("session", case, another, draft.submission_digest);
    assert!(
        matches!(
            result,
            Err(ApplicationError::Hearing(HearingError::ContextConflict))
        ),
        "{result:?}"
    );
    unchanged(&mut db, captured);
    db.store()
        .replace_administration(
            actor,
            case,
            CaseRevisionExpectation::new(1),
            case_stage_database_support::creation("Current administrative revision")
                .into_values()
                .editable()
                .clone(),
            at,
        )
        .unwrap();
    let cancelled = persist(
        &workflow,
        case,
        HearingCommand {
            operation_id: HearingOperationId::new(),
            hearing_id: recorded.snapshot.id,
            change: HearingChange::Cancel {
                expected_revision: recorded.snapshot.revision,
                reason: HearingNote::new("Date withdrawn after stage transition").unwrap(),
            },
        },
    );
    assert_eq!(cancelled.snapshot.status, HearingStatus::Cancelled);
    assert_eq!(cancelled.snapshot.values, recorded.snapshot.values);
    assert_eq!(
        cancelled.snapshot.scheduling_context,
        recorded.snapshot.scheduling_context
    );
    assert_eq!(cancelled.snapshot.recorded_administration_revision.get(), 2);
    assert_ne!(
        cancelled.snapshot.recorded_administration_digest,
        recorded.snapshot.recorded_administration_digest
    );
    assert_eq!(
        workflow
            .get(
                "session",
                case,
                recorded.snapshot.id,
                Some(recorded.snapshot.revision)
            )
            .unwrap(),
        recorded
    );
}

#[test]
fn typed_participant_keeps_the_exact_identity_even_after_identity_edit_and_archive() {
    use application::typed_participants::*;
    use typed_participant_service_support as typed;
    let Some(mut db) = Fixture::new() else { return };
    complete(&mut db);
    let record = typed::upload(&db, db.case, "identity.pdf");
    let identities = typed::service(&db, typed::FormatCheck(None));
    let request = typed::reviewed(
        identities
            .review_participant("session", db.case, typed::proposal(&record))
            .unwrap(),
    );
    let participant = identities
        .submit_participant(
            "session",
            db.case,
            ParticipantSubmission {
                prepared: request,
                signature: None,
            },
        )
        .unwrap();
    let subject = participant.bound_subject.as_ref().unwrap();
    let ParticipantRevisionSnapshot::Typed(ref revision) = participant.revision else {
        panic!("typed revision required")
    };
    let workflow = service(&db, db.owner, Role::Owner);
    let mut command = schedule();
    let HearingChange::Schedule { values, .. } = &mut command.change else {
        unreachable!()
    };
    *values = HearingValues::new(HearingValuesInput {
        kind: values.kind(),
        scheduled_at: values.scheduled_at(),
        modality: values.modality(),
        venue: values.venue().clone(),
        note: None,
        participants: vec![HearingParticipantRef::new(revision.id, revision.revision)],
        conviction_basis: None,
    })
    .unwrap();
    let captured = persist(&workflow, db.case, command);
    let values = SubjectValues::natural_person(
        RepresentedName::Known(ParticipantText::new("Updated represented identity").unwrap()),
        Declared::Unknown(ParticipantReason::new("Not declared").unwrap()),
        subject.values.identity_support().clone(),
    );
    let review = identities
        .review_subject(
            "session",
            db.case,
            subject.id,
            subject.revision,
            values.clone(),
        )
        .unwrap();
    assert!(review.candidates.is_empty());
    identities
        .replace_subject(
            "session",
            db.case,
            SubjectReplacementRequest {
                id: subject.id,
                expected_revision: subject.revision,
                values,
                review: IdentityReviewSubmission {
                    directory_stamp: review.directory_stamp,
                    different: vec![],
                    selection_reason: ParticipantReason::new("Identity reviewed").unwrap(),
                },
            },
        )
        .unwrap();
    participant_store(&db)
        .change_status(
            db.owner,
            db.case,
            revision.id,
            revision.revision,
            DirectoryStatus::Archived,
            db.at,
        )
        .unwrap();
    let retained = persist(&workflow, db.case, replacement(&captured));
    assert_eq!(retained.participants, captured.participants);
    let original = &retained.participants[0].overview;
    assert_eq!(original.display_name, "Ana");
    assert_eq!(original.subject, Some(revision.values.subject()));
    assert_eq!(original.subject.unwrap().revision, subject.revision);
    assert_eq!(original.kind, Some(revision.values.kind()));
}
