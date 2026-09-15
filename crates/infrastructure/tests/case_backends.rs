mod case_administration_support;

use application::cases::CaseRepository;
use application::ApplicationError;
use case_administration_support::Fixture;
use domain::cases::{CaseId, CaseMetadata};
use domain::identity::UserId;
use infrastructure::{PostgresCaseRepository, RingSha256Hasher};
use std::sync::{Arc, Barrier};

fn store(f: &Fixture) -> PostgresCaseRepository {
    PostgresCaseRepository::open(&f.runtime_url, Arc::new(RingSha256Hasher)).unwrap()
}
fn user(f: &mut Fixture, active: bool) -> UserId {
    let id = UserId::new();
    f.admin.execute("INSERT INTO users(id,email,password_hash,role,active,protected_totp_secret,recovery_codes) VALUES($1,$2,'fixture','litigator',$3,'\\x00','{}')", &[&id.as_uuid(),&format!("{id}@example.test"),&active]).unwrap();
    id
}
fn create(f: &Fixture, s: &PostgresCaseRepository, actor: UserId) -> CaseId {
    let id = CaseId::new();
    s.create_basic(
        actor,
        id,
        CaseMetadata::new("Evidence review", "Investigation 42").unwrap(),
        f.at,
    )
    .unwrap();
    id
}
#[test]
fn postgres_persists_cases_and_filters_membership_after_reconnection() {
    let Some(mut f) = Fixture::new() else { return };
    let creator = user(&mut f, true);
    let outsider = user(&mut f, true);
    let first = store(&f);
    let id = create(&f, &first, creator);
    let foreign = create(&f, &first, outsider);
    drop(first);
    let reopened = store(&f);
    let visible = reopened.list_basic(creator, 20, 0, f.at).unwrap();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].id, id);
    let stored = reopened.get_basic(f.owner, id, f.at).unwrap();
    assert_eq!(stored.title, "Evidence review");
    assert_eq!(stored.reference, "Investigation 42");
    assert_eq!(stored.created_by, creator);
    for missing in [foreign, CaseId::new()] {
        assert!(matches!(
            reopened.get_basic(creator, missing, f.at),
            Err(ApplicationError::CaseNotFound)
        ));
    }
}
#[test]
fn postgres_filters_before_applying_stable_pagination() {
    let Some(mut f) = Fixture::new() else { return };
    let creator = user(&mut f, true);
    let outsider = user(&mut f, true);
    let s = store(&f);
    let mut ids = Vec::new();
    for _ in 0..3 {
        ids.push(create(&f, &s, creator));
        create(&f, &s, outsider);
    }
    ids.sort_by_key(|id| id.as_uuid());
    for (offset, id) in ids.iter().enumerate() {
        let page = s.list_basic(creator, 1, offset as u32, f.at).unwrap();
        assert_eq!(page.len(), 1);
        assert_eq!(page[0].id, *id);
    }
    assert!(s.list_basic(creator, 1, 3, f.at).unwrap().is_empty());
}
#[test]
fn postgres_revokes_membership_on_the_next_read_and_allows_idempotent_changes() {
    let Some(mut f) = Fixture::new() else { return };
    let creator = user(&mut f, true);
    let member = user(&mut f, true);
    let s = store(&f);
    let id = create(&f, &s, creator);
    s.add_member(id, member, f.owner, f.at).unwrap();
    s.add_member(id, member, f.owner, f.at).unwrap();
    assert_eq!(s.list_basic(member, 10, 0, f.at).unwrap().len(), 1);
    s.get_basic(member, id, f.at).unwrap();
    s.remove_member(id, member, f.owner, f.at).unwrap();
    s.remove_member(id, member, f.owner, f.at).unwrap();
    assert!(matches!(
        s.get_basic(member, id, f.at),
        Err(ApplicationError::CaseNotFound)
    ));
    assert!(s.list_basic(member, 10, 0, f.at).unwrap().is_empty());
    s.get_basic(f.owner, id, f.at).unwrap();
}
#[test]
fn postgres_rolls_back_cases_without_a_valid_creator() {
    let Some(mut f) = Fixture::new() else { return };
    let inactive = user(&mut f, false);
    let s = store(&f);
    for actor in [UserId::new(), inactive] {
        let id = CaseId::new();
        assert!(matches!(
            s.create_basic(actor, id, CaseMetadata::new("New", "REF").unwrap(), f.at),
            Err(ApplicationError::InvalidSession)
        ));
        assert!(matches!(
            s.get_basic(f.owner, id, f.at),
            Err(ApplicationError::CaseNotFound)
        ));
    }
}
#[test]
fn postgres_rolls_back_case_creation_when_creator_membership_cannot_be_stored() {
    let Some(mut f) = Fixture::new() else { return };
    let creator = user(&mut f, true);
    let s = store(&f);
    let id = CaseId::new();
    f.admin
        .batch_execute(&format!(
            "ALTER TABLE case_memberships ADD CONSTRAINT reject_case CHECK(case_id<>'{id}')"
        ))
        .unwrap();
    assert!(matches!(
        s.create_basic(creator, id, CaseMetadata::new("New", "REF").unwrap(), f.at),
        Err(ApplicationError::Port(_))
    ));
    assert!(matches!(
        s.get_basic(f.owner, id, f.at),
        Err(ApplicationError::CaseNotFound)
    ));
    assert!(s.list_basic(creator, 10, 0, f.at).unwrap().is_empty());
}
#[test]
fn postgres_rejects_invalid_assignments_without_partial_memberships() {
    let Some(mut f) = Fixture::new() else { return };
    let inactive = user(&mut f, false);
    let creator = user(&mut f, true);
    let s = store(&f);
    let id = create(&f, &s, creator);
    for target in [UserId::new(), inactive] {
        assert!(matches!(
            s.add_member(id, target, f.owner, f.at),
            Err(ApplicationError::UserNotFound)
        ));
        let count: i64 = f
            .admin
            .query_one(
                "SELECT count(*) FROM case_memberships WHERE case_id=$1 AND user_id=$2",
                &[&id.as_uuid(), &target.as_uuid()],
            )
            .unwrap()
            .get(0);
        assert_eq!(count, 0);
    }
    assert!(matches!(
        s.add_member(CaseId::new(), creator, f.owner, f.at),
        Err(ApplicationError::CaseNotFound)
    ));
    assert!(matches!(
        s.remove_member(CaseId::new(), creator, f.owner, f.at),
        Err(ApplicationError::CaseNotFound)
    ));
}
#[test]
fn postgres_concurrent_assignments_preserve_one_membership() {
    let Some(mut f) = Fixture::new() else { return };
    let creator = user(&mut f, true);
    let member = user(&mut f, true);
    let s = store(&f);
    let id = create(&f, &s, creator);
    let barrier = Arc::new(Barrier::new(4));
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let adapter = store(&f);
            let barrier = barrier.clone();
            let owner = f.owner;
            let at = f.at;
            std::thread::spawn(move || {
                barrier.wait();
                adapter.add_member(id, member, owner, at).unwrap();
            })
        })
        .collect();
    for w in workers {
        w.join().unwrap();
    }
    let count: i64 = f
        .admin
        .query_one(
            "SELECT COUNT(*) FROM case_memberships WHERE case_id=$1 AND user_id=$2",
            &[&id.as_uuid(), &member.as_uuid()],
        )
        .unwrap()
        .get(0);
    assert_eq!(count, 1);
    let count: i64 = f
        .admin
        .query_one(
            "SELECT COUNT(*) FROM audit_events WHERE action='case.member_assigned' AND resource=$1",
            &[&format!("case:{id}:user:{member}")],
        )
        .unwrap()
        .get(0);
    assert_eq!(count, 1);
}
