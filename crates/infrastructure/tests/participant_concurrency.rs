#[allow(dead_code)]
mod participant_database_support;

use application::participants::{
    DirectoryStatus, ParticipantId, ParticipantRevision, ParticipantStore,
};
use application::ApplicationError;
use participant_database_support::{values, Fixture};
use postgres::{Client, NoTls};
use std::sync::{mpsc, Arc, Barrier};
use std::time::{Duration, Instant};

#[test]
fn competing_full_edit_and_status_change_have_exactly_one_audited_winner() {
    let Some(mut f) = Fixture::new() else { return };
    let store = f.store();
    let id = ParticipantId::new();
    store
        .create(f.owner, f.case, id, values("Original"), f.at)
        .unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let mut workers = Vec::new();
    for status_only in [true, false] {
        let writer = f.store();
        let gate = barrier.clone();
        let (actor, case, at) = (f.owner, f.case, f.at);
        workers.push(std::thread::spawn(move || {
            gate.wait();
            if status_only {
                writer.change_status(
                    actor,
                    case,
                    id,
                    ParticipantRevision::initial(),
                    DirectoryStatus::Archived,
                    at,
                )
            } else {
                writer
                    .replace(
                        actor,
                        case,
                        id,
                        ParticipantRevision::initial(),
                        values("Concurrent text"),
                        at,
                    )
                    .map(Into::into)
            }
        }));
    }
    barrier.wait();
    let results = workers
        .into_iter()
        .map(|w| w.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, Err(ApplicationError::ParticipantRevisionConflict)))
            .count(),
        1
    );
    let count:i64=f.admin.query_one("SELECT COUNT(*) FROM audit_events WHERE action IN ('participant.updated','participant.directory_status_changed')", &[]).unwrap().get(0);
    assert_eq!(count, 1);
    let current = store
        .get(f.owner, f.case, id, f.at)
        .unwrap()
        .into_manual()
        .unwrap();
    assert_eq!(current.revision.get(), 2);
    let retried = store
        .change_status(
            f.owner,
            f.case,
            id,
            current.revision,
            DirectoryStatus::Archived,
            f.at,
        )
        .unwrap();
    assert_eq!(
        retried.manual().unwrap().values.display_name(),
        current.values.display_name()
    );
}

#[test]
fn both_status_and_text_commit_orders_preserve_explicit_edit_semantics() {
    for status_first in [true, false] {
        let Some(f) = Fixture::new() else { return };
        let writer = f.store();
        let other = f.store();
        let id = ParticipantId::new();
        writer
            .create(f.owner, f.case, id, values("Original"), f.at)
            .unwrap();
        let first = if status_first {
            writer.change_status(
                f.owner,
                f.case,
                id,
                ParticipantRevision::initial(),
                DirectoryStatus::Archived,
                f.at,
            )
        } else {
            writer
                .replace(
                    f.owner,
                    f.case,
                    id,
                    ParticipantRevision::initial(),
                    values("Current text"),
                    f.at,
                )
                .map(Into::into)
        }
        .unwrap()
        .into_manual()
        .unwrap();
        assert!(matches!(
            other.change_status(
                f.owner,
                f.case,
                id,
                ParticipantRevision::initial(),
                DirectoryStatus::Archived,
                f.at
            ),
            Err(ApplicationError::ParticipantRevisionConflict)
        ));
        let final_snapshot = if status_first {
            other
                .replace(
                    f.owner,
                    f.case,
                    id,
                    first.revision,
                    values("Current text").with_directory_status(DirectoryStatus::Archived),
                    f.at,
                )
                .map(Into::into)
        } else {
            other.change_status(
                f.owner,
                f.case,
                id,
                first.revision,
                DirectoryStatus::Archived,
                f.at,
            )
        }
        .unwrap()
        .into_manual()
        .unwrap();
        assert_eq!(final_snapshot.values.display_name(), "Current text");
        assert_eq!(
            final_snapshot.values.directory_status(),
            DirectoryStatus::Archived
        );
        assert_eq!(final_snapshot.revision.get(), 3);
    }
}

#[test]
fn membership_and_role_revocation_committed_while_waiting_prevent_mutation() {
    for revoke in ["membership", "role", "active"] {
        let Some(mut f) = Fixture::new() else { return };
        let actor = f.user("litigator", true);
        let store = f.store();
        let id = ParticipantId::new();
        store
            .create(f.owner, f.case, id, values("Protected"), f.at)
            .unwrap();
        let mut admin = Client::connect(&f.admin_url, NoTls).unwrap();
        let mut transaction = admin.transaction().unwrap();
        transaction
            .query_one("SELECT pg_advisory_xact_lock(280603412820)", &[])
            .unwrap();
        let sql = match revoke {
            "membership" => "DELETE FROM case_memberships WHERE user_id=$1",
            "role" => "UPDATE users SET role='client' WHERE id=$1",
            _ => "UPDATE users SET active=false WHERE id=$1",
        };
        transaction.execute(sql, &[&actor.as_uuid()]).unwrap();
        let (case, at) = (f.case, f.at);
        let writer = std::thread::spawn(move || {
            store.change_status(
                actor,
                case,
                id,
                ParticipantRevision::initial(),
                DirectoryStatus::Archived,
                at,
            )
        });
        wait_for_lock(&mut f.control, &f.role);
        transaction.commit().unwrap();
        let before = f.snapshot();
        let result = writer.join().unwrap();
        assert!(match revoke {
            "membership" => matches!(result, Err(ApplicationError::ParticipantNotFound)),
            "role" => matches!(result, Err(ApplicationError::PermissionDenied)),
            _ => matches!(result, Err(ApplicationError::InvalidSession)),
        });
        assert_eq!(f.snapshot(), before);
    }
}

#[test]
fn direct_insert_waits_for_predecessor_commit_and_rejects_rolled_back_predecessor() {
    for commit in [true, false] {
        let Some(mut f) = Fixture::new() else { return };
        let mut client = Client::connect(&f.runtime_url, NoTls).unwrap();
        let mut first = client.transaction().unwrap();
        let id = uuid::Uuid::new_v4();
        first
            .execute(
                "INSERT INTO case_participants(id,case_id) VALUES($1,$2)",
                &[&id, &f.case.as_uuid()],
            )
            .unwrap();
        insert(&mut first, id, f.owner.as_uuid(), 1).unwrap();
        let (ready, observed) = mpsc::channel();
        let (url, actor) = (f.runtime_url.clone(), f.owner.as_uuid());
        let writer = std::thread::spawn(move || {
            let mut c = Client::connect(&url, NoTls).unwrap();
            ready.send(()).unwrap();
            insert(&mut c, id, actor, 2)
        });
        observed.recv_timeout(Duration::from_secs(5)).unwrap();
        wait_for_lock(&mut f.control, &f.role);
        if commit {
            first.commit().unwrap();
        } else {
            first.rollback().unwrap();
        }
        assert_eq!(writer.join().unwrap().is_ok(), commit);
        assert_eq!(
            f.admin
                .query_one("SELECT COUNT(*) FROM case_participant_revisions", &[])
                .unwrap()
                .get::<_, i64>(0),
            if commit { 2 } else { 0 }
        );
    }
}

fn insert<C: postgres::GenericClient>(
    c: &mut C,
    id: uuid::Uuid,
    actor: uuid::Uuid,
    revision: i64,
) -> Result<u64, postgres::Error> {
    c.execute("INSERT INTO case_participant_revisions(participant_id,revision,display_name,procedural_role,directory_status,values_digest,changed_at,changed_by,changed_by_email)
        VALUES($1,$2,'Person','Witness','active',pg_catalog.sha256(participant_values_bytes('Person','Witness',NULL,NULL,'active')),'2025-01-01T00:00:00Z',$3,'author@example.test')",&[&id,&revision,&actor])
}
fn wait_for_lock(control: &mut Client, role: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if control.query_one("SELECT EXISTS(SELECT 1 FROM pg_locks l JOIN pg_stat_activity a ON a.pid=l.pid WHERE a.usename=$1 AND l.locktype='advisory' AND NOT l.granted)",&[&role]).unwrap().get::<_,bool>(0) {return;}
        assert!(
            Instant::now() < deadline,
            "participant operation never waited for audit lock"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
