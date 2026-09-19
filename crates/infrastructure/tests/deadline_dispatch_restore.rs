mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_dispatch_guard_support;
#[allow(dead_code)]
mod deadline_dispatch_support;
mod deadline_profile_database_support;
mod procedural_fact_backend_support;

use application::deadline_dispatch::DeadlineDispatchStream;
use deadline_backend_support as dl;
use deadline_dispatch_guard_support as guards;
use deadline_dispatch_support as dispatch;
use postgres::{Client, NoTls};
use std::process::Command;

#[test]
fn dump_restore_reopens_partial_dispatch_without_migration_and_resumes_stable_jobs() {
    let Some((mut db, changed, _)) = guards::setup(&[0, 10, 20, 30, 40]) else {
        return;
    };
    let sequence = dispatch::event_sequence(&mut db, &changed);
    let store = dispatch::open(&db);
    let events = dispatch::dispatch(&store, DeadlineDispatchStream::Events, 2);
    assert_eq!((events.selected, events.inserted), (2, 2));
    assert!(!events.completed_scan);
    assert_eq!(events.event.as_ref().unwrap().sequence, sequence);
    assert_eq!(events.progress.event.active_sequence, Some(sequence));
    assert_eq!(
        events.progress.event.after_deadline_id,
        Some(dispatch::id(10))
    );
    let bootstrap = dispatch::dispatch(&store, DeadlineDispatchStream::LegacyBootstrap, 2);
    assert_eq!((bootstrap.selected, bootstrap.inserted), (2, 2));
    assert!(!bootstrap.completed_scan);
    assert_eq!(bootstrap.progress.event, events.progress.event);
    assert_eq!(
        bootstrap.progress.bootstrap_after_deadline_id,
        Some(dispatch::id(10))
    );
    let partial_ids = vec![dispatch::id(0), dispatch::id(10)];
    assert_eq!(dispatch::event_jobs(&mut db, sequence), partial_ids);
    assert_eq!(dispatch::bootstrap_jobs(&mut db), partial_ids);
    drop(store);

    let before = dispatch::snapshot(&mut db);
    let original_jobs = before["jobs"].as_array().unwrap();
    let original_audit = before["audit"].as_array().unwrap();
    assert!(!original_jobs.is_empty());
    assert!(!original_audit.is_empty());
    let history_before = dispatch::deadline_history(&mut db);
    dump_restore_with_empty_search_path(&mut db);
    assert_eq!(dispatch::snapshot(&mut db), before);
    assert_eq!(dispatch::deadline_history(&mut db), history_before);

    // Reopening must validate the restored schema and ACLs without repairing them.
    let restored = dispatch::open(&db);
    assert_eq!(dispatch::snapshot(&mut db), before);
    let mut bootstrap_after = Some(dispatch::id(10));
    for (count, after) in [(2, Some(30)), (1, None)] {
        let event_page = dispatch::dispatch(&restored, DeadlineDispatchStream::Events, 2);
        assert_eq!(event_page.event.as_ref().unwrap().sequence, sequence);
        assert_eq!((event_page.selected, event_page.inserted), (count, count));
        assert_eq!(event_page.completed_scan, after.is_none());
        assert_eq!(
            event_page.progress.event.active_sequence,
            after.map(|_| sequence)
        );
        assert_eq!(
            event_page.progress.event.after_deadline_id,
            after.map(dispatch::id)
        );
        assert_eq!(
            event_page.progress.bootstrap_after_deadline_id,
            bootstrap_after
        );
        if after.is_none() {
            assert_eq!(event_page.progress.event.completed_sequence, Some(sequence));
        }
        let bootstrap_page =
            dispatch::dispatch(&restored, DeadlineDispatchStream::LegacyBootstrap, 2);
        assert_eq!(
            (bootstrap_page.selected, bootstrap_page.inserted),
            (count, count)
        );
        assert_eq!(bootstrap_page.completed_scan, after.is_none());
        assert_eq!(bootstrap_page.progress.event, event_page.progress.event);
        bootstrap_after = after.map(dispatch::id);
        assert_eq!(
            bootstrap_page.progress.bootstrap_after_deadline_id,
            bootstrap_after
        );
    }
    let expected_ids = [0, 10, 20, 30, 40]
        .into_iter()
        .map(dispatch::id)
        .collect::<Vec<_>>();
    assert_eq!(dispatch::event_jobs(&mut db, sequence), expected_ids);
    assert_eq!(dispatch::bootstrap_jobs(&mut db), expected_ids);
    assert_eq!(dispatch::deadline_history(&mut db), history_before);
    let after = dispatch::snapshot(&mut db);
    let restored_jobs = after["jobs"].as_array().unwrap();
    assert_eq!(restored_jobs.len(), original_jobs.len() + 6);
    for original in original_jobs {
        assert!(
            restored_jobs.contains(original),
            "restoration or resume changed a prior job"
        );
    }
    let restored_audit = after["audit"].as_array().unwrap();
    assert_eq!(restored_audit.len(), original_audit.len() + 4);
    assert_eq!(
        &restored_audit[..original_audit.len()],
        original_audit.as_slice()
    );
    let counts = db
        .admin
        .query_one(
            "SELECT count(*),count(DISTINCT id),count(DISTINCT operation_id),
            count(DISTINCT (deadline_id,event_sequence,bootstrap_policy_version))
        FROM deadline_reevaluation_jobs",
            &[],
        )
        .unwrap();
    let expected_count = i64::try_from(restored_jobs.len()).unwrap();
    for column in 0..4_usize {
        assert_eq!(counts.get::<_, i64>(column), expected_count);
    }
    let idle_event = dispatch::dispatch(&restored, DeadlineDispatchStream::Events, 2);
    assert!(idle_event.event.is_none());
    assert_eq!((idle_event.selected, idle_event.inserted), (0, 0));
    let idle_bootstrap = dispatch::dispatch(&restored, DeadlineDispatchStream::LegacyBootstrap, 2);
    assert_eq!((idle_bootstrap.selected, idle_bootstrap.inserted), (0, 0));
    assert_eq!(dispatch::snapshot(&mut db), after);
    drop(restored);
    let reopened = dispatch::open(&db);
    assert_eq!(dispatch::snapshot(&mut db), after);
    drop(reopened);
}

fn dump_restore_with_empty_search_path(db: &mut dl::Fixture) {
    let directory = tempfile::tempdir().unwrap();
    let dump = directory.path().join("deadline-dispatch.dump");
    run(Command::new("pg_dump")
        .args([
            "--dbname",
            &db.admin_url,
            "--schema",
            &db.schema,
            "--format=custom",
            "--file",
        ])
        .arg(&dump));
    let mut restore_url = reqwest::Url::parse(&db.admin_url).unwrap();
    let remaining = restore_url
        .query_pairs()
        .filter(|(name, _)| name != "options")
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    restore_url
        .query_pairs_mut()
        .clear()
        .extend_pairs(remaining)
        .append_pair("options", "-csearch_path=");
    let mut probe = Client::connect(restore_url.as_str(), NoTls).unwrap();
    let search_path: String = probe.query_one("SHOW search_path", &[]).unwrap().get(0);
    assert_eq!(
        search_path, "",
        "restore connection must have an empty search path"
    );
    drop(probe);
    db.control
        .batch_execute(&format!("DROP SCHEMA {} CASCADE", db.schema))
        .unwrap();
    run(Command::new("pg_restore")
        .args(["--exit-on-error", "--dbname", restore_url.as_str()])
        .arg(&dump));
}

fn run(command: &mut Command) {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
