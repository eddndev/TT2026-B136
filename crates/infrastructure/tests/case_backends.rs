use std::{env, sync::Arc, thread};

use application::cases::{CaseAccess, CaseRecord, CaseRepository};
use application::identity::{UserRecord, UserRepository};
use application::ApplicationError;
use domain::cases::CaseId;
use domain::crypto::{RecoveryCodeSet, RECOVERY_CODE_COUNT};
use domain::identity::{Role, UserId};
use infrastructure::{PostgresCaseRepository, PostgresUserRepository};
use postgres::{Client, NoTls};

fn database_url() -> Option<String> {
    let url = env::var("CASE_TEST_DATABASE_URL").ok();
    if url.is_none() {
        eprintln!("skipping PostgreSQL case test: CASE_TEST_DATABASE_URL is unset");
    }
    url
}

fn user(database_url: &str, active: bool) -> UserId {
    let id = UserId::new();
    PostgresUserRepository::connect(database_url)
        .unwrap()
        .insert(UserRecord {
            id,
            email: format!("{id}@example.com"),
            password_hash: "$argon2id$test".into(),
            role: Role::Litigator,
            active,
            protected_totp_secret: vec![7; 48],
            recovery_codes: RecoveryCodeSet::from_hashes(
                (0..RECOVERY_CODE_COUNT)
                    .map(|index| format!("$argon2id$recovery-{index}"))
                    .collect(),
            )
            .unwrap(),
            revision: 0,
        })
        .unwrap();
    id
}

fn record(created_by: UserId) -> CaseRecord {
    CaseRecord {
        id: CaseId::new(),
        title: "Evidence review".into(),
        reference: "Investigation 42".into(),
        created_by,
    }
}

#[test]
fn postgres_persists_cases_and_filters_membership_after_reconnection() {
    let Some(url) = database_url() else {
        return;
    };
    let repository = PostgresCaseRepository::connect(&url).unwrap();
    let creator = user(&url, true);
    let outsider = user(&url, true);
    let case = record(creator);
    let foreign = record(outsider);
    repository.insert(case.clone()).unwrap();
    repository.insert(foreign.clone()).unwrap();
    drop(repository);

    let reopened = PostgresCaseRepository::connect(&url).unwrap();
    let visible = reopened.list(CaseAccess::Assigned(creator), 20, 0).unwrap();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].id, case.id);
    let stored = reopened.find(case.id, CaseAccess::All).unwrap().unwrap();
    assert_eq!(stored.title, case.title);
    assert_eq!(stored.reference, case.reference);
    assert_eq!(stored.created_by, creator);
    assert!(reopened
        .find(foreign.id, CaseAccess::Assigned(creator))
        .unwrap()
        .is_none());
    assert!(reopened
        .find(CaseId::new(), CaseAccess::Assigned(creator))
        .unwrap()
        .is_none());
}

#[test]
fn postgres_filters_before_applying_stable_pagination() {
    let Some(url) = database_url() else {
        return;
    };
    let repository = PostgresCaseRepository::connect(&url).unwrap();
    let creator = user(&url, true);
    let outsider = user(&url, true);
    let mut ids = Vec::new();
    for _ in 0..3 {
        let case = record(creator);
        ids.push(case.id);
        repository.insert(case).unwrap();
        repository.insert(record(outsider)).unwrap();
    }
    ids.sort_by_key(|id| id.as_uuid());
    for (offset, expected) in ids.into_iter().enumerate() {
        let page = repository
            .list(CaseAccess::Assigned(creator), 1, offset as u32)
            .unwrap();
        assert_eq!(page.len(), 1);
        assert_eq!(page[0].id, expected);
    }
    assert!(repository
        .list(CaseAccess::Assigned(creator), 1, 3)
        .unwrap()
        .is_empty());
}

#[test]
fn postgres_revokes_membership_on_the_next_read_and_allows_idempotent_changes() {
    let Some(url) = database_url() else {
        return;
    };
    let repository = PostgresCaseRepository::connect(&url).unwrap();
    let creator = user(&url, true);
    let member = user(&url, true);
    let case = record(creator);
    repository.insert(case.clone()).unwrap();
    repository.add_member(case.id, member).unwrap();
    repository.add_member(case.id, member).unwrap();
    assert_eq!(
        repository
            .list(CaseAccess::Assigned(member), 10, 0)
            .unwrap()
            .len(),
        1
    );
    assert!(repository
        .find(case.id, CaseAccess::Assigned(member))
        .unwrap()
        .is_some());
    repository.remove_member(case.id, member).unwrap();
    repository.remove_member(case.id, member).unwrap();
    assert!(repository
        .find(case.id, CaseAccess::Assigned(member))
        .unwrap()
        .is_none());
    assert!(repository
        .list(CaseAccess::Assigned(member), 10, 0)
        .unwrap()
        .is_empty());
    assert!(repository.find(case.id, CaseAccess::All).unwrap().is_some());
}

#[test]
fn postgres_rolls_back_cases_without_a_valid_creator() {
    let Some(url) = database_url() else {
        return;
    };
    let repository = PostgresCaseRepository::connect(&url).unwrap();
    for creator in [UserId::new(), user(&url, false)] {
        let case = record(creator);
        assert!(matches!(
            repository.insert(case.clone()),
            Err(ApplicationError::UserNotFound)
        ));
        assert!(repository.find(case.id, CaseAccess::All).unwrap().is_none());
    }
}

#[test]
fn postgres_rolls_back_case_creation_when_creator_membership_cannot_be_stored() {
    let Some(url) = database_url() else {
        return;
    };
    let repository = PostgresCaseRepository::connect(&url).unwrap();
    let creator = user(&url, true);
    let case = record(creator);
    let mut client = Client::connect(&url, NoTls).unwrap();
    let constraint_name = format!("reject_case_{}", case.id.as_uuid().simple());
    client
        .batch_execute(&format!(
            "ALTER TABLE case_memberships ADD CONSTRAINT {constraint_name}
             CHECK (case_id <> '{}')",
            case.id
        ))
        .unwrap();
    let result = repository.insert(case.clone());
    client
        .batch_execute(&format!(
            "ALTER TABLE case_memberships DROP CONSTRAINT {constraint_name}"
        ))
        .unwrap();
    assert!(matches!(result, Err(ApplicationError::Port(_))));
    assert!(repository.find(case.id, CaseAccess::All).unwrap().is_none());
    assert!(repository
        .list(CaseAccess::Assigned(creator), 10, 0)
        .unwrap()
        .is_empty());
}

#[test]
fn postgres_rejects_invalid_assignments_without_partial_memberships() {
    let Some(url) = database_url() else {
        return;
    };
    let repository = PostgresCaseRepository::connect(&url).unwrap();
    let creator = user(&url, true);
    let case = record(creator);
    repository.insert(case.clone()).unwrap();
    for target in [UserId::new(), user(&url, false)] {
        assert!(matches!(
            repository.add_member(case.id, target),
            Err(ApplicationError::UserNotFound)
        ));
        assert!(repository
            .find(case.id, CaseAccess::Assigned(target))
            .unwrap()
            .is_none());
    }
    assert!(matches!(
        repository.add_member(CaseId::new(), creator),
        Err(ApplicationError::CaseNotFound)
    ));
    assert!(matches!(
        repository.remove_member(CaseId::new(), creator),
        Err(ApplicationError::CaseNotFound)
    ));
}

#[test]
fn postgres_concurrent_assignments_preserve_one_membership() {
    let Some(url) = database_url() else {
        return;
    };
    let repository = PostgresCaseRepository::connect(&url).unwrap();
    let creator = user(&url, true);
    let member = user(&url, true);
    let case = record(creator);
    repository.insert(case.clone()).unwrap();
    let barrier = Arc::new(std::sync::Barrier::new(4));
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let adapter = PostgresCaseRepository::connect(&url).unwrap();
            let barrier = barrier.clone();
            let id = case.id;
            thread::spawn(move || {
                barrier.wait();
                adapter.add_member(id, member).unwrap();
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    let mut client = Client::connect(&url, NoTls).unwrap();
    let count: i64 = client
        .query_one(
            "SELECT COUNT(*) FROM case_memberships WHERE case_id=$1 AND user_id=$2",
            &[&case.id.as_uuid(), &member.as_uuid()],
        )
        .unwrap()
        .get(0);
    assert_eq!(count, 1);
}
