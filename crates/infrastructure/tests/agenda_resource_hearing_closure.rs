use super::*;
use application::{
    participants::{DirectoryStatus, ParticipantId, ParticipantStore, ParticipantValues},
    resource_activities::ResourceActivityId,
};
use domain::resource_hearings::{ResourceHearingId, ResourceHearingOperationId};
use infrastructure::{PostgresParticipantStore, RingSha256Hasher};
use std::sync::Arc;

#[test]
fn all_and_resource_agendas_keep_two_authors_captures_after_case_closure() {
    let Some(mut db) = own::Fixture::new() else {
        return;
    };
    let (captures, mut command) = own::setup(&mut db);
    let people =
        PostgresParticipantStore::open(&db.runtime_url, Arc::new(RingSha256Hasher)).unwrap();
    let person = people
        .create(
            db.owner,
            db.case,
            ParticipantId::new(),
            ParticipantValues::new(
                "Original person",
                "Witness",
                None,
                None,
                DirectoryStatus::Active,
            )
            .unwrap(),
            db.at,
        )
        .unwrap();
    let values = &command.values;
    command.values = ResourceHearingValues::new(ResourceHearingValuesInput {
        kind: values.kind(),
        scheduled_at: values.scheduled_at(),
        modality: values.modality(),
        venue: values.venue().clone(),
        note: values.note().cloned(),
        participants: vec![HearingParticipantRef::new(person.id, person.revision)],
        scheduling_basis: values.scheduling_basis().clone(),
    })
    .unwrap();
    let first = own::submit(&db, command.clone());
    let user = db.user("litigator", true);
    command.hearing_id = ResourceHearingId::new();
    command.operation_id = ResourceHearingOperationId::new();
    command.association_id = ResourceActivityId::new();
    command.act = None;
    let start = command.values.scheduled_at().utc();
    let second_at = start + Duration::hours(36);
    at(&mut command, second_at);
    let workflow = own::service(&db, user, Role::Litigator);
    let draft = workflow
        .prepare("session", db.case, command.resource.id, command.clone())
        .unwrap();
    let second = workflow
        .submit(
            "session",
            db.case,
            command.resource.id,
            command,
            draft.submission_digest,
        )
        .unwrap();
    assert_eq!(first.hearing.review.recorded_by.id, db.owner);
    assert_eq!(second.hearing.review.recorded_by.id, user);
    let expected = [&first, &second];
    let store = agenda::store(&db);
    let compare = |status| {
        let read = |kind| {
            store
                .list(
                    db.owner,
                    AgendaQuery::new(
                        20,
                        start,
                        second_at + Duration::seconds(1),
                        kind,
                        HearingStatusFilter::All,
                        None,
                    )
                    .unwrap(),
                )
                .unwrap()
        };
        let page = read(AgendaKind::ResourceHearing);
        assert_eq!(read(AgendaKind::All), page);
        assert!(page.complete && page.next_after.is_none());
        assert_eq!(page.items.len(), 2);
        let mut captured = Vec::new();
        for (item, original) in page.items.iter().zip(expected) {
            let AgendaItem::ResourceHearing { case, hearing } = item else {
                panic!("expected a resource hearing capture")
            };
            assert_eq!(case.case_id, db.case);
            assert_eq!(case.status, status);
            assert_eq!(hearing.revision.get(), 1);
            assert_eq!(hearing.id, original.origin.hearing_id);
            assert_eq!(hearing.resource_id, captures.resource.id);
            assert_eq!(hearing.association_id, original.origin.association_id);
            assert_eq!(hearing.capture_digest, original.origin.capture_digest);
            assert_eq!(hearing.participant_count, 1);
            captured.push(hearing.clone());
        }
        captured
    };
    let before = compare(CaseAdministrativeStatus::Active);
    people
        .replace(
            db.owner,
            db.case,
            person.id,
            person.revision,
            ParticipantValues::new(
                "Updated person",
                "Witness",
                None,
                None,
                DirectoryStatus::Active,
            )
            .unwrap(),
            db.at,
        )
        .unwrap();
    crate::resource_activity_support::persist(
        &crate::resource_activity_support::service(&db, db.owner, Role::Owner),
        db.case,
        captures.resource.id,
        crate::resource_activity_support::unlink(&first.association, captures.head.revision),
    );
    captures.archive(&db);
    assert_eq!(compare(CaseAdministrativeStatus::Active), before);
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(1),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    assert_eq!(compare(CaseAdministrativeStatus::Closed), before);
    assert_eq!(agenda::audits(&mut db), 6);
}
