use super::*;
use std::process::Command;

#[test]
fn dump_restore_reopens_runtime_with_exact_hearing_prefixes_and_original_replay() {
    let Some(mut db) = Fixture::new() else { return };
    let (actor, command) = setup(&mut db);
    let first = persist(&db, actor.clone(), command.clone());
    let second = persist(&db, actor.clone(), replacement(&first));
    let third = persist(&db, actor.clone(), cancellation(&second));
    let before = snapshot(&mut db);
    let directory = tempfile::tempdir().unwrap();
    let dump = directory.path().join("precautionary-hearings.dump");
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
    db.control
        .batch_execute(&format!("DROP SCHEMA {} CASCADE", db.schema))
        .unwrap();
    run(Command::new("pg_restore")
        .args(["--exit-on-error", "--dbname", &db.admin_url])
        .arg(&dump));
    assert_eq!(snapshot(&mut db), before);

    // Open the restored catalog directly, without migrating or repairing it.
    let reopened = reads(&db, actor.clone());
    assert_eq!(
        reopened
            .get("session", db.case, command.hearing_id, None)
            .unwrap(),
        third
    );
    for expected in [&first, &second, &third] {
        assert_eq!(
            reopened
                .get(
                    "session",
                    db.case,
                    command.hearing_id,
                    Some(expected.capture.review.result_revision),
                )
                .unwrap(),
            *expected
        );
        assert_eq!(
            reopened
                .get_operation(
                    "session",
                    db.case,
                    expected.capture.review.command.operation_id,
                )
                .unwrap(),
            *expected
        );
    }
    let replay = service_with_format(
        &db,
        actor,
        FormatCheck(Some(Box::new(|| panic!("replay must skip admission")))),
    );
    assert_eq!(
        replay
            .submit(
                "session",
                db.case,
                command,
                confirmation(&first.capture.review),
            )
            .unwrap(),
        first
    );
}

fn run(command: &mut Command) {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
