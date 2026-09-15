mod document_store_support;

use std::sync::mpsc;
use std::time::{Duration, Instant};

use application::cases::CaseRepository;
use application::documents::{CaseDocumentStore, DocumentAction, DocumentQuery};
use application::ApplicationError;
use domain::identity::Role;
use infrastructure::{PostgresCaseDocumentStore, PostgresCaseRepository};
use postgres::{Client, NoTls};
use time::OffsetDateTime;

use document_store_support::{case, database_url, document, evidence, user};

#[test]
fn metadata_reads_commit_their_audit_before_returning_and_serialize_membership_revocation() {
    let Some(url) = database_url() else { return };
    for action in [DocumentAction::List, DocumentAction::Read] {
        let owner = user(&url, Role::Owner);
        let actor = user(&url, Role::Litigator);
        let case_id = case(&url, actor);
        let store = PostgresCaseDocumentStore::connect(&url).unwrap();
        let mut record = document();
        let at = OffsetDateTime::now_utc();
        store.insert(actor, case_id, record.clone(), at).unwrap();
        record.seal(evidence()).unwrap();
        store.seal(actor, case_id, record.clone(), at).unwrap();
        let reader = PostgresCaseDocumentStore::connect(&url).unwrap();
        let revoker = PostgresCaseRepository::connect(&url).unwrap();
        let suffix = record.id.as_uuid().simple().to_string();
        let hold_key = i64::from(
            u32::from_be_bytes(record.id.as_uuid().as_bytes()[..4].try_into().unwrap())
                & 0x7fff_ffff,
        );
        let mut admin = Client::connect(&url, NoTls).unwrap();
        admin
            .batch_execute(&format!(
                "CREATE FUNCTION pause_metadata_{suffix}() RETURNS TRIGGER LANGUAGE plpgsql AS $$
             BEGIN PERFORM pg_advisory_xact_lock({hold_key}); RETURN NEW; END; $$;
             CREATE TRIGGER pause_metadata_{suffix} BEFORE INSERT ON audit_events FOR EACH ROW
             WHEN (NEW.action='{}' AND NEW.resource LIKE 'case:{case_id}:%')
             EXECUTE FUNCTION pause_metadata_{suffix}();",
                action.audit_action()
            ))
            .unwrap();
        let mut blocker = Client::connect(&url, NoTls).unwrap();
        let mut blocked = blocker.transaction().unwrap();
        blocked
            .query_one("SELECT pg_advisory_xact_lock($1)", &[&hold_key])
            .unwrap();
        let (read_done, read_result) = mpsc::channel();
        let id = record.id;
        let reading = std::thread::spawn(move || {
            let result = match action {
                DocumentAction::List => reader
                    .list(
                        actor,
                        case_id,
                        DocumentQuery::new(1, 0, None, None).unwrap(),
                        at,
                    )
                    .map(|page| page.documents[0].document.id),
                _ => reader
                    .get(
                        actor,
                        case_id,
                        id,
                        application::documents::VersionSelection::Current,
                        at,
                    )
                    .map(|summary| summary.document.id),
            };
            read_done.send(result).unwrap();
        });
        wait_for_guarded_audit(&mut admin, hold_key);
        let early_read = read_result.recv_timeout(Duration::from_millis(100));
        let (revoke_done, revoke_result) = mpsc::channel();
        let revoking = std::thread::spawn(move || {
            revoke_done
                .send(revoker.remove_member(case_id, actor, owner))
                .unwrap();
        });
        let early_revoke = revoke_result.recv_timeout(Duration::from_millis(100));
        blocked.commit().unwrap();
        reading.join().unwrap();
        revoking.join().unwrap();
        admin.batch_execute(&format!(
            "DROP TRIGGER pause_metadata_{suffix} ON audit_events; DROP FUNCTION pause_metadata_{suffix}();"
        )).unwrap();
        assert!(matches!(early_read, Err(mpsc::RecvTimeoutError::Timeout)));
        assert!(matches!(early_revoke, Err(mpsc::RecvTimeoutError::Timeout)));
        assert_eq!(
            read_result
                .recv_timeout(Duration::from_secs(1))
                .unwrap()
                .unwrap(),
            id
        );
        revoke_result
            .recv_timeout(Duration::from_secs(1))
            .unwrap()
            .unwrap();
        assert!(matches!(
            store.get(
                actor,
                case_id,
                id,
                application::documents::VersionSelection::Current,
                at
            ),
            Err(ApplicationError::DocumentNotFound(_))
        ));
        assert!(matches!(
            store.list(
                actor,
                case_id,
                DocumentQuery::new(1, 0, None, None).unwrap(),
                at
            ),
            Err(ApplicationError::CaseNotFound)
        ));
        let entries = store.audit_entries(owner).unwrap();
        let read_event = entries
            .iter()
            .find(|entry| {
                entry.event.action == action.audit_action()
                    && entry
                        .event
                        .resource
                        .starts_with(&format!("case:{case_id}:"))
            })
            .unwrap();
        let revoked = entries
            .iter()
            .find(|entry| {
                entry.event.action == "case.member_removed"
                    && entry.event.resource.contains(&case_id.to_string())
            })
            .unwrap();
        assert!(read_event.event.sequence < revoked.event.sequence);
    }
}

fn wait_for_guarded_audit(admin: &mut Client, hold_key: i64) {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let waiting: bool = admin
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory'
             AND classid=0 AND objid::bigint=$1 AND NOT granted)",
                &[&hold_key],
            )
            .unwrap()
            .get(0);
        if waiting {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "read never reached its guarded audit insert"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
