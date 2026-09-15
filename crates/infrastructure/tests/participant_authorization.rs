#[allow(dead_code)]
mod participant_database_support;

use application::participants::{
    DirectoryStatus, ParticipantHistoryQuery, ParticipantId, ParticipantQuery, ParticipantRevision,
    ParticipantStatusFilter, ParticipantStore,
};
use application::ApplicationError;
use domain::cases::CaseId;
use domain::identity::UserId;
use infrastructure::PostgresParticipantStore;
use participant_database_support::{values, Fixture};
use time::OffsetDateTime;

#[test]
fn current_roles_and_memberships_control_every_directory_operation() {
    let Some(mut f) = Fixture::new() else { return };
    let store = f.store();
    let id = ParticipantId::new();
    store
        .create(f.owner, f.case, id, values("Person"), f.at)
        .unwrap();
    for role in ["litigator", "paralegal", "client"] {
        let actor = f.user(role, true);
        if role == "client" {
            let before = f.snapshot();
            for result in reads(&store, actor, f.case, id, f.at)
                .into_iter()
                .chain(writes(&store, actor, f.case, id, f.at))
            {
                assert!(matches!(result, Err(ApplicationError::PermissionDenied)));
            }
            assert_eq!(f.snapshot(), before);
        } else {
            assert!(reads(&store, actor, f.case, id, f.at)
                .into_iter()
                .all(|r| r.is_ok()));
            if role == "paralegal" {
                let before = f.snapshot();
                assert!(writes(&store, actor, f.case, id, f.at)
                    .into_iter()
                    .all(|r| matches!(r, Err(ApplicationError::PermissionDenied))));
                assert_eq!(f.snapshot(), before);
            } else {
                let current = store.get(actor, f.case, id, f.at).unwrap();
                store
                    .replace(actor, f.case, id, current.revision, values("Edited"), f.at)
                    .unwrap();
            }
            f.admin
                .execute(
                    "DELETE FROM case_memberships WHERE user_id=$1",
                    &[&actor.as_uuid()],
                )
                .unwrap();
            let before = f.snapshot();
            assert!(matches!(
                store.get(actor, f.case, id, f.at),
                Err(ApplicationError::ParticipantNotFound)
            ));
            assert!(matches!(
                store.list(actor, f.case, query(), f.at),
                Err(ApplicationError::CaseNotFound)
            ));
            if role == "litigator" {
                assert!(matches!(
                    store.replace(
                        actor,
                        f.case,
                        id,
                        ParticipantRevision::initial(),
                        values("Denied"),
                        f.at
                    ),
                    Err(ApplicationError::ParticipantNotFound)
                ));
                assert!(matches!(
                    store.create(actor, f.case, ParticipantId::new(), values("Denied"), f.at),
                    Err(ApplicationError::CaseNotFound)
                ));
            }
            assert_eq!(f.snapshot(), before);
        }
    }
}

#[test]
fn owner_cannot_redirect_participant_to_another_case_and_inactive_actor_is_rejected() {
    let Some(mut f) = Fixture::new() else { return };
    let store = f.store();
    let id = ParticipantId::new();
    store
        .create(f.owner, f.case, id, values("Scoped"), f.at)
        .unwrap();
    let other = CaseId::new();
    f.admin
        .execute(
            "INSERT INTO cases(id,title,reference,created_by) VALUES($1,'Other','Other',$2)",
            &[&other.as_uuid(), &f.owner.as_uuid()],
        )
        .unwrap();
    for case in [other, CaseId::new()] {
        for participant in [id, ParticipantId::new()] {
            let before = f.snapshot();
            let results = [
                store.get(f.owner, case, participant, f.at).map(|_| ()),
                store
                    .history(
                        f.owner,
                        case,
                        participant,
                        ParticipantHistoryQuery::new(1, None).unwrap(),
                        f.at,
                    )
                    .map(|_| ()),
                store
                    .replace(
                        f.owner,
                        case,
                        participant,
                        ParticipantRevision::initial(),
                        values("Denied"),
                        f.at,
                    )
                    .map(|_| ()),
                store
                    .change_status(
                        f.owner,
                        case,
                        participant,
                        ParticipantRevision::initial(),
                        DirectoryStatus::Archived,
                        f.at,
                    )
                    .map(|_| ()),
            ];
            assert!(results
                .into_iter()
                .all(|r| matches!(r, Err(ApplicationError::ParticipantNotFound))));
            assert_eq!(f.snapshot(), before);
        }
    }
    f.admin
        .execute(
            "UPDATE users SET active=false WHERE id=$1",
            &[&f.owner.as_uuid()],
        )
        .unwrap();
    let before = f.snapshot();
    assert!(reads(&store, f.owner, f.case, id, f.at)
        .into_iter()
        .chain(writes(&store, f.owner, f.case, id, f.at))
        .all(|r| matches!(r, Err(ApplicationError::InvalidSession))));
    assert_eq!(f.snapshot(), before);
}

fn query() -> ParticipantQuery {
    ParticipantQuery::new(10, None, None, None, ParticipantStatusFilter::All).unwrap()
}
fn reads(
    s: &PostgresParticipantStore,
    a: UserId,
    c: CaseId,
    id: ParticipantId,
    at: OffsetDateTime,
) -> [Result<(), ApplicationError>; 3] {
    [
        s.get(a, c, id, at).map(|_| ()),
        s.list(a, c, query(), at).map(|_| ()),
        s.history(
            a,
            c,
            id,
            ParticipantHistoryQuery::new(10, None).unwrap(),
            at,
        )
        .map(|_| ()),
    ]
}
fn writes(
    s: &PostgresParticipantStore,
    a: UserId,
    c: CaseId,
    id: ParticipantId,
    at: OffsetDateTime,
) -> [Result<(), ApplicationError>; 3] {
    [
        s.create(a, c, ParticipantId::new(), values("New"), at)
            .map(|_| ()),
        s.replace(
            a,
            c,
            id,
            ParticipantRevision::initial(),
            values("Changed"),
            at,
        )
        .map(|_| ()),
        s.change_status(
            a,
            c,
            id,
            ParticipantRevision::initial(),
            DirectoryStatus::Archived,
            at,
        )
        .map(|_| ()),
    ]
}
