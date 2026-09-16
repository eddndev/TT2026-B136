use super::hearing_result_database_support::{persist, record, service, values, Fixture};
use super::hearing_result_revalidation_support::*;
use super::typed_participant_service_support as typed;
use application::{hearing_results::*, participants::*, typed_participants::*};
use domain::identity::Role;
use std::sync::Arc;

fn participant_store(db: &Fixture) -> infrastructure::PostgresParticipantStore {
    infrastructure::PostgresParticipantStore::open(
        &db.runtime_url,
        Arc::new(infrastructure::RingSha256Hasher),
    )
    .unwrap()
}
fn select(command: &mut HearingResultCommand, id: ParticipantId, revision: ParticipantRevision) {
    let mut selected = input(&values("Declared attendance"));
    selected.attendees = vec![HearingResultAttendee::new(
        id,
        revision,
        HearingResultCapacity::new("Declared witness").unwrap(),
        None,
    )];
    set_values(command, HearingResultValues::new(selected).unwrap());
}

#[test]
fn exact_attendee_survives_concurrent_edit_and_archive_and_archived_revision_is_selectable() {
    let Some(mut db) = Fixture::new() else { return };
    let anchor = appointment(&mut db);
    let participants = participant_store(&db);
    let first = participants
        .create(
            db.owner,
            db.case,
            ParticipantId::new(),
            ParticipantValues::new(
                "Original witness",
                "Witness",
                None,
                None,
                DirectoryStatus::Active,
            )
            .unwrap(),
            db.at,
        )
        .unwrap();
    let workflow = service(&db, db.owner, Role::Owner);
    let mut command = record(anchor.snapshot.id);
    select(&mut command, first.id, first.revision);
    let draft = workflow
        .prepare("session", db.case, command.clone())
        .unwrap();
    let (actor, case, at) = (db.owner, db.case, db.at);
    let changed = hooked(&db, actor, Role::Owner, move || {
        let edited = participants
            .replace(
                actor,
                case,
                first.id,
                first.revision,
                ParticipantValues::new(
                    "Current witness",
                    "Witness",
                    None,
                    None,
                    DirectoryStatus::Active,
                )
                .unwrap(),
                at,
            )
            .unwrap();
        participants
            .change_status(
                actor,
                case,
                edited.id,
                edited.revision,
                DirectoryStatus::Archived,
                at,
            )
            .unwrap();
    });
    let exact = changed
        .submit("session", case, command, draft.submission_digest)
        .unwrap();
    assert_eq!(
        exact.attendees[0].participant.overview.display_name,
        "Original witness"
    );
    assert_eq!(
        exact.attendees[0].participant.overview.revision,
        first.revision
    );
    let mut archived = record(anchor.snapshot.id);
    select(
        &mut archived,
        first.id,
        ParticipantRevision::new(3).unwrap(),
    );
    let selected = persist(&workflow, case, archived);
    assert_eq!(
        selected.attendees[0].participant.overview.directory_status,
        DirectoryStatus::Archived
    );
    assert_eq!(
        selected.attendees[0].participant.overview.display_name,
        "Current witness"
    );
}

#[test]
fn typed_attendee_preserves_subject_digest_after_concurrent_identity_edit_and_archive() {
    let Some(mut db) = Fixture::new() else { return };
    let anchor = appointment(&mut db);
    let support = typed::upload(&db, db.case, "identity.pdf");
    let identities = typed::service(&db, typed::FormatCheck(None));
    let prepared = typed::reviewed(
        identities
            .review_participant("session", db.case, typed::proposal(&support))
            .unwrap(),
    );
    let person = identities
        .submit_participant(
            "session",
            db.case,
            ParticipantSubmission {
                prepared,
                signature: None,
            },
        )
        .unwrap();
    let subject = person.bound_subject.as_ref().unwrap().clone();
    let ParticipantRevisionSnapshot::Typed(revision) = person.revision else {
        panic!("typed revision required")
    };
    let mut command = record(anchor.snapshot.id);
    select(&mut command, revision.id, revision.revision);
    let draft = service(&db, db.owner, Role::Owner)
        .prepare("session", db.case, command.clone())
        .unwrap();
    let participants = participant_store(&db);
    let (actor, case, at) = (db.owner, db.case, db.at);
    let subject_id = subject.id;
    let subject_revision = subject.revision;
    let replacement = SubjectValues::natural_person(
        RepresentedName::Known(ParticipantText::new("Updated represented identity").unwrap()),
        Declared::Unknown(ParticipantReason::new("Not declared").unwrap()),
        subject.values.identity_support().clone(),
    );
    let workflow = hooked(&db, actor, Role::Owner, move || {
        let review = identities
            .review_subject(
                "session",
                case,
                subject_id,
                subject_revision,
                replacement.clone(),
            )
            .unwrap();
        assert!(review.candidates.is_empty());
        identities
            .replace_subject(
                "session",
                case,
                SubjectReplacementRequest {
                    id: subject_id,
                    expected_revision: subject_revision,
                    values: replacement.clone(),
                    review: IdentityReviewSubmission {
                        directory_stamp: review.directory_stamp,
                        different: vec![],
                        selection_reason: ParticipantReason::new("Identity reviewed").unwrap(),
                    },
                },
            )
            .unwrap();
        participants
            .change_status(
                actor,
                case,
                revision.id,
                revision.revision,
                DirectoryStatus::Archived,
                at,
            )
            .unwrap();
    });
    let result = workflow
        .submit("session", case, command, draft.submission_digest)
        .unwrap();
    assert_eq!(result.attendees, draft.attendees);
    let selected = &result.attendees[0];
    assert_eq!(selected.participant.overview.display_name, "Ana");
    assert_eq!(selected.subject_digest, Some(subject.values_digest));
    assert_eq!(
        selected.participant.overview.subject.unwrap().revision,
        subject_revision
    );
}
