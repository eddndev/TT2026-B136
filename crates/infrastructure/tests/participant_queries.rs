#[allow(dead_code)]
mod participant_database_support;

use application::participants::{
    DirectoryStatus, ParticipantHistoryQuery, ParticipantId, ParticipantQuery, ParticipantRevision,
    ParticipantStatusFilter, ParticipantStore, ParticipantValues,
};
use participant_database_support::{values, Fixture};

#[test]
fn latest_values_are_filtered_literally_before_case_local_keyset_pagination() {
    let Some(mut f) = Fixture::new() else { return };
    let store = f.store();
    let ids = (1..=4)
        .map(|i| ParticipantId::from_uuid(uuid::Uuid::from_u128(i)))
        .collect::<Vec<_>>();
    for (id, name) in ids
        .iter()
        .zip(["Old", "Match %_\\", "Match lower", "Match Last"])
    {
        store
            .create(f.owner, f.case, *id, values(name), f.at)
            .unwrap();
    }
    store
        .replace(
            f.owner,
            f.case,
            ids[0],
            ParticipantRevision::initial(),
            values("Match First"),
            f.at,
        )
        .unwrap();
    store
        .change_status(
            f.owner,
            f.case,
            ids[1],
            ParticipantRevision::initial(),
            DirectoryStatus::Archived,
            f.at,
        )
        .unwrap();
    let query = |limit, cursor, name, role, status| {
        ParticipantQuery::new(limit, cursor, name, role, status).unwrap()
    };
    assert!(store
        .list(
            f.owner,
            f.case,
            query(10, None, Some("Old"), None, ParticipantStatusFilter::All),
            f.at
        )
        .unwrap()
        .participants
        .is_empty());
    let first = store
        .list(
            f.owner,
            f.case,
            query(
                1,
                None,
                Some("Match"),
                Some("Witness"),
                ParticipantStatusFilter::Active,
            ),
            f.at,
        )
        .unwrap();
    assert_eq!(first.participants[0].id, ids[0]);
    assert!(first.has_more);
    assert_eq!(first.next_after_id, Some(ids[0]));
    let next = store
        .list(
            f.owner,
            f.case,
            query(
                2,
                first.next_after_id,
                Some("Match"),
                Some("Witness"),
                ParticipantStatusFilter::Active,
            ),
            f.at,
        )
        .unwrap();
    assert_eq!(
        next.participants.iter().map(|p| p.id).collect::<Vec<_>>(),
        vec![ids[2], ids[3]]
    );
    assert!(!next.has_more);
    assert_eq!(next.next_after_id, None);
    let literal = store
        .list(
            f.owner,
            f.case,
            query(10, None, Some("%_\\"), None, ParticipantStatusFilter::All),
            f.at,
        )
        .unwrap();
    assert_eq!(literal.participants.len(), 1);
    assert_eq!(literal.participants[0].id, ids[1]);
    assert!(store
        .list(
            f.owner,
            f.case,
            query(10, None, Some("match"), None, ParticipantStatusFilter::All),
            f.at
        )
        .unwrap()
        .participants
        .is_empty());
    assert!(store
        .list(
            f.owner,
            f.case,
            query(
                10,
                None,
                None,
                Some("witness"),
                ParticipantStatusFilter::All
            ),
            f.at
        )
        .unwrap()
        .participants
        .is_empty());
    let beyond = ParticipantId::from_uuid(uuid::Uuid::from_u128(900));
    assert!(store
        .list(
            f.owner,
            f.case,
            query(10, Some(beyond), None, None, ParticipantStatusFilter::All),
            f.at
        )
        .unwrap()
        .participants
        .is_empty());
    let other = domain::cases::CaseId::new();
    f.admin
        .execute(
            "INSERT INTO cases(id,title,reference,created_by) VALUES($1,'Other','Other',$2)",
            &[&other.as_uuid(), &f.owner.as_uuid()],
        )
        .unwrap();
    assert!(store
        .list(
            f.owner,
            other,
            query(10, None, None, None, ParticipantStatusFilter::All),
            f.at
        )
        .unwrap()
        .participants
        .is_empty());
}

#[test]
fn history_cursor_and_unicode_filters_preserve_exact_representations() {
    let Some(f) = Fixture::new() else { return };
    let store = f.store();
    let id = ParticipantId::new();
    let first = ParticipantValues::new("Jos\u{e9}", "R\u{e9}", None, None, DirectoryStatus::Active)
        .unwrap();
    store
        .create(f.owner, f.case, id, first.clone(), f.at)
        .unwrap();
    let second = ParticipantValues::new(
        "Jose\u{301}",
        "Re\u{301}",
        None,
        None,
        DirectoryStatus::Active,
    )
    .unwrap();
    store
        .replace(
            f.owner,
            f.case,
            id,
            ParticipantRevision::initial(),
            second.clone(),
            f.at,
        )
        .unwrap();
    let query = ParticipantQuery::new(
        10,
        None,
        Some(first.display_name()),
        None,
        ParticipantStatusFilter::All,
    )
    .unwrap();
    assert!(store
        .list(f.owner, f.case, query, f.at)
        .unwrap()
        .participants
        .is_empty());
    let query = ParticipantQuery::new(
        10,
        None,
        Some(second.display_name()),
        Some(second.procedural_role()),
        ParticipantStatusFilter::All,
    )
    .unwrap();
    assert_eq!(
        store
            .list(f.owner, f.case, query, f.at)
            .unwrap()
            .participants
            .len(),
        1
    );
    let empty = store
        .history(
            f.owner,
            f.case,
            id,
            ParticipantHistoryQuery::new(1, Some(1)).unwrap(),
            f.at,
        )
        .unwrap();
    assert!(empty.revisions.is_empty());
    assert!(!empty.has_more);
    assert_eq!(empty.next_before_revision, None);
}
