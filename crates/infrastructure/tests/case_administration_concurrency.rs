mod case_administration_support;
use application::cases::*;
use application::ApplicationError;
use case_administration_support::Fixture;
use domain::cases::{CaseId, CaseMetadata};
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

fn profile(nuc: &str) -> PenalCaseProfile {
    PenalCaseProfile::new(nuc, "Office", nuc, "Court", &["Reported"], None, None).unwrap()
}
fn wait_for_lock(f: &mut Fixture) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let waiting:bool=f.admin.query_one("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE usename=$1 AND wait_event='advisory')", &[&f.role]).unwrap().get(0);
        if waiting {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "case operation did not wait for audit lock"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn concurrent_replacements_have_one_successor_and_one_generic_revision_conflict() {
    let Some(mut f) = Fixture::new() else { return };
    let store = f.store();
    let id = CaseId::new();
    store
        .create_basic(f.owner, id, CaseMetadata::new("Base", "REF").unwrap(), f.at)
        .unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let workers: Vec<_> = ["First", "Second"]
        .into_iter()
        .map(|title| {
            let store = f.store();
            let barrier = barrier.clone();
            let actor = f.owner;
            let at = f.at;
            std::thread::spawn(move || {
                barrier.wait();
                store.replace_administration(
                    actor,
                    id,
                    CaseRevisionExpectation::new(1),
                    CaseEditableValues::new(CaseMetadata::new(title, "REF").unwrap(), None),
                    at,
                )
            })
        })
        .collect();
    let results: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Err(ApplicationError::CaseRevisionConflict)))
            .count(),
        1
    );
    let count: i64 = f
        .admin
        .query_one(
            "SELECT count(*) FROM case_administration_revisions WHERE case_id=$1",
            &[&id.as_uuid()],
        )
        .unwrap()
        .get(0);
    assert_eq!(count, 2);
}
#[test]
fn concurrent_complete_registrations_cannot_claim_the_same_current_identifiers() {
    let Some(mut f) = Fixture::new() else { return };
    let barrier = Arc::new(Barrier::new(2));
    let workers: Vec<_> = (0..2)
        .map(|_| {
            let store = f.store();
            let barrier = barrier.clone();
            let actor = f.owner;
            let at = f.at;
            std::thread::spawn(move || {
                barrier.wait();
                store.register_penal(
                    actor,
                    CaseId::new(),
                    PenalCaseCreation::new(
                        CaseMetadata::new("New", "REF").unwrap(),
                        profile("Occupied"),
                    ),
                    at,
                )
            })
        })
        .collect();
    let results: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Err(ApplicationError::CaseIdentifierConflict)))
            .count(),
        1
    );
    let counts=f.admin.query_one("SELECT (SELECT count(*) FROM case_administration_revisions),(SELECT count(*) FROM case_initial_stage_registrations),(SELECT count(*) FROM audit_events)",&[]).unwrap();
    assert_eq!(counts.get::<_, i64>(0), 1);
    assert_eq!(counts.get::<_, i64>(1), 1);
    assert_eq!(counts.get::<_, i64>(2), 4);
}
#[test]
fn waiting_basic_reads_observe_committed_revocation_or_disabled_actor_before_returning_data() {
    for revoke in [true, false] {
        let Some(mut f) = Fixture::new() else { return };
        let actor = f.user("client", true);
        f.admin
            .batch_execute(&format!(
                "ALTER ROLE {} SET default_transaction_isolation='repeatable read'",
                f.role
            ))
            .unwrap();
        let reader = f.store();
        f.admin
            .batch_execute("SELECT pg_advisory_lock(280603412820)")
            .unwrap();
        let id = f.case;
        let at = f.at;
        let worker = std::thread::spawn(move || reader.get_basic(actor, id, at));
        wait_for_lock(&mut f);
        if revoke {
            f.admin
                .execute(
                    "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
                    &[&id.as_uuid(), &actor.as_uuid()],
                )
                .unwrap();
        } else {
            f.admin
                .execute(
                    "UPDATE users SET active=FALSE WHERE id=$1",
                    &[&actor.as_uuid()],
                )
                .unwrap();
        }
        f.admin
            .batch_execute("SELECT pg_advisory_unlock(280603412820)")
            .unwrap();
        let result = worker.join().unwrap();
        if revoke {
            assert!(matches!(result, Err(ApplicationError::CaseNotFound)));
        } else {
            assert!(matches!(result, Err(ApplicationError::InvalidSession)));
        }
        let count: i64 = f
            .admin
            .query_one("SELECT count(*) FROM audit_events", &[])
            .unwrap()
            .get(0);
        assert_eq!(count, 0);
    }
}
#[test]
fn direct_read_committed_inserts_recheck_uniqueness_after_waiting_for_another_transaction() {
    let Some(mut f) = Fixture::new() else { return };
    let other = CaseId::new();
    f.admin.execute("INSERT INTO cases(id,title,reference,created_by,required_initial_revision) VALUES($1,'Baseline','REF-OLD',$2,NULL)", &[&other.as_uuid(),&f.owner.as_uuid()]).unwrap();
    let sql="INSERT INTO case_administration_revisions(case_id,revision,title,reference,administrative_status,nuc,nuc_authority,judicial_case_number,judicial_authority,offenses,values_digest,changed_at,changed_by,changed_by_email) VALUES($1,1,'Baseline','REF-OLD','active','Same','Office','Same','Court',ARRAY['Reported'],sha256(case_administration_bytes('active','Baseline','REF-OLD','Same','Office','Same','Court',ARRAY['Reported'],NULL,NULL)),'2025-01-01T00:00:00Z',$2,'owner@example.test')";
    let mut runtime = f.runtime();
    let mut first = f.admin.transaction().unwrap();
    first
        .execute(sql, &[&f.case.as_uuid(), &f.owner.as_uuid()])
        .unwrap();
    let owner = f.owner;
    let worker =
        std::thread::spawn(move || runtime.execute(sql, &[&other.as_uuid(), &owner.as_uuid()]));
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let waiting:bool=first.query_one("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE usename=$1 AND wait_event='advisory')", &[&f.role]).unwrap().get(0);
        if waiting {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    first.commit().unwrap();
    let error = worker.join().unwrap().unwrap_err();
    assert_eq!(error.code().unwrap().code(), "23505");
    let count: i64 = f
        .admin
        .query_one("SELECT count(*) FROM case_administration_revisions", &[])
        .unwrap()
        .get(0);
    assert_eq!(count, 1);
}
