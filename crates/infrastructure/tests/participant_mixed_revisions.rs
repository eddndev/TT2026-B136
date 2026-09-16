mod case_administration_support;
mod typed_participant_database_support;

use application::{participants::*, ApplicationError};
use case_administration_support::Fixture;
use infrastructure::{PostgresParticipantStore, RingSha256Hasher};
use std::sync::Arc;
use typed_participant_database_support::Bundle;

fn populated() -> Option<(Fixture, Bundle, PostgresParticipantStore)> {
    let db = Fixture::new()?;
    let mut bundle = Bundle::new(&db);
    let mut client = postgres::Client::connect(&db.admin_url, postgres::NoTls).unwrap();
    let mut tx = client.transaction().unwrap();
    bundle.seed_document(&mut tx, &db);
    bundle.seed_manual(&mut tx, &db);
    bundle.refresh_stamp(&db);
    bundle.insert(&mut tx, &db);
    tx.commit().unwrap();
    let store =
        PostgresParticipantStore::open(&db.runtime_url, Arc::new(RingSha256Hasher)).unwrap();
    Some((db, bundle, store))
}

#[test]
fn current_exact_and_paginated_history_select_both_revision_families() {
    let Some((db, b, store)) = populated() else {
        return;
    };
    let current = store.get(db.owner, db.case, b.participant, db.at).unwrap();
    assert_eq!(current.revision_number().get(), 2);
    assert!(matches!(
        current.revision,
        ParticipantRevisionSnapshot::Typed(_)
    ));
    assert_eq!(current.bound_subject.unwrap().id, b.subject);
    let first = store
        .get_revision(
            db.owner,
            db.case,
            b.participant,
            ParticipantRevision::initial(),
            db.at,
        )
        .unwrap();
    assert_eq!(
        first.manual().unwrap().values.procedural_role(),
        "Declared role"
    );
    let page = store
        .history(
            db.owner,
            db.case,
            b.participant,
            ParticipantHistoryQuery::new(1, None).unwrap(),
            db.at,
        )
        .unwrap();
    assert_eq!(page.revisions[0].revision_number().get(), 2);
    assert!(page.has_more);
    assert_eq!(page.next_before_revision.unwrap().get(), 2);
    let old = store
        .history(
            db.owner,
            db.case,
            b.participant,
            ParticipantHistoryQuery::new(1, Some(2)).unwrap(),
            db.at,
        )
        .unwrap();
    assert_eq!(old.revisions[0].revision_number().get(), 1);
    assert!(!old.has_more);
}

#[test]
fn listing_filters_the_union_head_without_resurrecting_manual_values() {
    let Some((db, b, store)) = populated() else {
        return;
    };
    let query =
        || ParticipantQuery::new(10, None, None, None, ParticipantStatusFilter::All).unwrap();
    let page = store.list(db.owner, db.case, query(), db.at).unwrap();
    assert_eq!(page.participants.len(), 1);
    assert_eq!(page.participants[0].revision.get(), 2);
    assert_eq!(page.participants[0].subject.unwrap().id, b.subject);
    let manual = query().with_profile_filter(None, ParticipantProfileFilter::Manual);
    assert!(store
        .list(db.owner, db.case, manual, db.at)
        .unwrap()
        .participants
        .is_empty());
    let stale_role = ParticipantQuery::new(
        10,
        None,
        None,
        Some("Declared role"),
        ParticipantStatusFilter::All,
    )
    .unwrap();
    assert!(store
        .list(db.owner, db.case, stale_role, db.at)
        .unwrap()
        .participants
        .is_empty());
    let typed = query().with_profile_filter(
        Some(domain::typed_participants::ParticipantKind::Defendant),
        ParticipantProfileFilter::Typed,
    );
    assert_eq!(
        store
            .list(db.owner, db.case, typed, db.at)
            .unwrap()
            .participants
            .len(),
        1
    );
}

#[test]
fn status_extends_typed_history_and_manual_replacement_rejects_without_writes() {
    let Some((mut db, b, store)) = populated() else {
        return;
    };
    let before = store.get(db.owner, db.case, b.participant, db.at).unwrap();
    let result = store.replace(
        db.owner,
        db.case,
        b.participant,
        ParticipantRevision::new(2).unwrap(),
        ParticipantValues::new("Wrong manual", "Other", None, None, DirectoryStatus::Active)
            .unwrap(),
        db.at,
    );
    assert!(matches!(
        result,
        Err(ApplicationError::ParticipantProfileRequired)
    ));
    assert_eq!(
        db.admin
            .query_one("SELECT count(*) FROM case_participant_revisions", &[])
            .unwrap()
            .get::<_, i64>(0),
        1
    );
    let archived = store
        .change_status(
            db.owner,
            db.case,
            b.participant,
            ParticipantRevision::new(2).unwrap(),
            DirectoryStatus::Archived,
            db.at,
        )
        .unwrap();
    assert_eq!(archived.revision_number().get(), 3);
    let (ParticipantRevisionSnapshot::Typed(old), ParticipantRevisionSnapshot::Typed(new)) =
        (before.revision, archived.revision)
    else {
        panic!("typed snapshots required")
    };
    assert_eq!(new.values.directory_status(), DirectoryStatus::Archived);
    assert_eq!(new.values.subject(), old.values.subject());
    assert_eq!(new.credential_origin, old.credential_origin);
    assert_eq!(new.submission_digest, old.submission_digest);
    assert_eq!(new.submission_revision, old.submission_revision);
    assert_ne!(new.values_digest, old.values_digest);
    assert_eq!(
        db.admin
            .query_one("SELECT count(*) FROM case_participant_typed_revisions", &[])
            .unwrap()
            .get::<_, i64>(0),
        2
    );
}
