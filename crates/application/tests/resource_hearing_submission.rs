use super::*;
use application::resource_hearings::*;
use domain::{clock::OffsetDateTime, crypto::Sha256Digest};

fn at() -> OffsetDateTime {
    case_support::instant()
}
fn prepared(f: &Fixture) -> PreparedResourceHearing {
    prepare_resource_hearing_change(
        hearing_support::hasher(),
        &f.actor,
        f.case(),
        f.resource(),
        f.command.clone(),
        f.material.clone(),
    )
    .unwrap()
}
fn result(f: &Fixture) -> ResourceHearingCreation {
    prepared(f).into_creation(at()).unwrap()
}
fn replay_store(value: ResourceHearingCreation) -> MockStore {
    let mut store = MockStore::new();
    store
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, _, _| Ok(ResourceHearingPreparation::Replay(Box::new(value))));
    store
}

#[test]
fn confirmed_submission_passes_exact_review_to_one_commit() {
    let f = Fixture::new(Role::Litigator);
    let draft = f.prepare().unwrap();
    let expected = draft.clone();
    let actor = f.actor.id;
    let case = f.case();
    let resource = f.resource();
    let mut store = f.store();
    store
        .expect_commit()
        .times(1)
        .return_once(move |a, c, r, prepared| {
            assert_eq!((a, c, r), (actor, case, resource));
            assert_eq!(prepared.draft(), &expected);
            prepared.into_creation(at())
        });
    let (identity, _) = procedural_resource_support::identity_for(f.actor.clone(), 3);
    let created = service(store, identity)
        .submit(
            "session",
            case,
            resource,
            f.command.clone(),
            draft.submission_digest,
        )
        .unwrap();
    assert_eq!(created.hearing.review, draft);
    assert_eq!(created.origin.association_id, f.command.association_id);
    assert_eq!(
        created.origin.capture_digest,
        created.hearing.capture_digest
    );
    assert_eq!(created.hearing.revision.get(), 1);
}

#[test]
fn changed_review_never_commits_and_exact_replay_never_writes() {
    let f = Fixture::new(Role::Owner);
    let digest = f.prepare().unwrap().submission_digest;
    let (identity, _) = procedural_resource_support::identity_for(f.actor.clone(), 1);
    assert!(service(f.store(), identity)
        .submit(
            "session",
            f.case(),
            f.resource(),
            f.command.clone(),
            Sha256Digest::from_array([4; 32])
        )
        .is_err());
    let saved = result(&f);
    let (identity, _) = procedural_resource_support::identity_for(f.actor.clone(), 2);
    assert_eq!(
        service(replay_store(saved.clone()), identity)
            .submit("session", f.case(), f.resource(), f.command.clone(), digest)
            .unwrap(),
        saved
    );
}

#[test]
fn stored_creation_rejects_changed_payload_marker_time_and_foreign_author() {
    let f = Fixture::new(Role::Owner);
    let saved = result(&f);
    let mut variants = Vec::new();
    let mut v = saved.clone();
    v.origin.association_id = ResourceActivityId::new();
    variants.push(v);
    let mut v = saved.clone();
    v.origin.submission_digest = Sha256Digest::from_array([3; 32]);
    variants.push(v);
    let mut v = saved.clone();
    v.hearing.recorded_at += time::Duration::seconds(1);
    variants.push(v);
    let mut v = saved.clone();
    v.hearing.review.command.hearing_id = ResourceHearingId::new();
    variants.push(v);
    let mut v = saved.clone();
    v.hearing.review.support.digest = Sha256Digest::from_array([2; 32]);
    variants.push(v);
    let mut v = saved.clone();
    v.hearing.material.resource_head.receipt.capture_digest = Sha256Digest::from_array([1; 32]);
    variants.push(v);
    for variant in variants {
        assert!(
            resource_hearing_creation_matches(hearing_support::hasher().as_ref(), &variant)
                .is_err()
        );
    }
    let mut other = f.clone();
    other.actor.id = domain::identity::UserId::new();
    let (identity, _) = procedural_resource_support::identity_for(other.actor.clone(), 1);
    assert!(service(replay_store(saved), identity)
        .submit(
            "session",
            f.case(),
            f.resource(),
            f.command.clone(),
            f.prepare().unwrap().submission_digest
        )
        .is_err());
}

#[test]
fn creation_clock_must_follow_every_captured_source_and_be_utc() {
    let f = Fixture::new(Role::Owner);
    assert!(prepared(&f)
        .into_creation(f.material.resource_head.recorded_at - time::Duration::seconds(1))
        .is_err());
    assert!(prepared(&f)
        .into_creation(at().to_offset(time::UtcOffset::from_hms(1, 0, 0).unwrap()))
        .is_err());
}

#[test]
fn authority_changes_before_commit_or_before_return_never_release_evidence() {
    for before_commit in [true, false] {
        let f = Fixture::new(Role::Owner);
        let digest = f.prepare().unwrap().submission_digest;
        let mut store = f.store();
        if !before_commit {
            store
                .expect_commit()
                .times(1)
                .return_once(|_, _, _, p| p.into_creation(at()));
        }
        let mut identity = case_support::MockIdentity::new();
        let mut sequence = mockall::Sequence::new();
        for index in 0..if before_commit { 2 } else { 3 } {
            let mut actor = f.actor.clone();
            if index == if before_commit { 1 } else { 2 } {
                actor.email = "changed@example.com".into();
            }
            identity
                .expect_authenticate()
                .times(1)
                .in_sequence(&mut sequence)
                .return_once(move |_| Ok(actor));
        }
        assert!(matches!(
            service(store, identity).submit("session", f.case(), f.resource(), f.command, digest),
            Err(ApplicationError::InvalidSession)
        ));
    }
}

#[test]
fn stored_response_must_be_the_exact_review_even_when_its_receipt_is_valid() {
    let f = Fixture::new(Role::Owner);
    let digest = f.prepare().unwrap().submission_digest;
    let mut another = f.clone();
    another.command.association_id = ResourceActivityId::new();
    let saved = result(&another);
    let mut store = f.store();
    store
        .expect_commit()
        .times(1)
        .return_once(move |_, _, _, _| Ok(saved));
    let (identity, _) = procedural_resource_support::identity_for(f.actor.clone(), 2);
    assert!(service(store, identity)
        .submit("session", f.case(), f.resource(), f.command, digest)
        .is_err());
}
