#[allow(dead_code)]
mod participant_database_support;

use application::participants::{
    participant_digest, DirectoryStatus, ParticipantId, ParticipantRevision, ParticipantStore,
};
use infrastructure::{initialize_database, RingSha256Hasher};
use participant_database_support::{values, Fixture};
use postgres::{Client, NoTls};

#[test]
fn participant_checks_and_sequence_function_work_with_empty_search_path() {
    let Some(mut f) = Fixture::new() else { return };
    let store = f.store();
    let id = ParticipantId::new();
    store
        .create(f.owner, f.case, id, values("Unicode \u{10000}"), f.at)
        .unwrap();
    let mut runtime = Client::connect(&f.runtime_url, NoTls).unwrap();
    runtime.batch_execute("SET search_path=''").unwrap();
    let v = values("Unicode \u{10000}");
    let bytes: Vec<u8> = runtime
        .query_one(
            &format!(
                "SELECT {}.participant_values_bytes($1,$2,$3,$4,$5)",
                f.schema
            ),
            &[
                &v.display_name(),
                &v.procedural_role(),
                &v.organization(),
                &v.legal_status(),
                &v.directory_status().as_str(),
            ],
        )
        .unwrap()
        .get(0);
    assert_eq!(bytes, v.canonical_bytes());
    let digest = participant_digest(&RingSha256Hasher::new(), &v);
    runtime.execute(&format!("INSERT INTO {}.case_participant_revisions(participant_id,revision,display_name,procedural_role,organization,directory_status,values_digest,changed_at,changed_by,changed_by_email)
        VALUES($1,2,$2,$3,$4,'active',$5,'2025-01-01T00:00:00Z',$6,'historical@example.test')",f.schema),
        &[&id.as_uuid(),&v.display_name(),&v.procedural_role(),&v.organization(),&&digest.as_bytes()[..],&f.owner.as_uuid()]).unwrap();
    let before = f.snapshot();
    assert!(runtime.execute(&format!("INSERT INTO {}.case_participant_revisions(participant_id,revision,display_name,procedural_role,directory_status,values_digest,changed_at,changed_by,changed_by_email)
        VALUES($1,3,'Bad','Witness','active',decode(repeat('00',32),'hex'),'2025-01-01T00:00:00Z',$2,'actor@example.test')",f.schema),&[&id.as_uuid(),&f.owner.as_uuid()]).is_err());
    assert_eq!(f.snapshot(), before);
}

#[test]
fn migration_rerun_and_real_dump_restore_preserve_directory_history_and_runtime_grants() {
    let Some(mut f) = Fixture::new() else { return };
    let store = f.store();
    let id = ParticipantId::new();
    store
        .create(f.owner, f.case, id, values("Before"), f.at)
        .unwrap();
    store
        .replace(
            f.owner,
            f.case,
            id,
            ParticipantRevision::initial(),
            values("After"),
            f.at,
        )
        .unwrap();
    store
        .change_status(
            f.owner,
            f.case,
            id,
            ParticipantRevision::new(2).unwrap(),
            DirectoryStatus::Archived,
            f.at,
        )
        .unwrap();
    drop(store);
    let before = f.snapshot();
    initialize_database(&f.admin_url, &f.role).unwrap();
    assert_eq!(f.snapshot(), before);
    let temp = tempfile::tempdir().unwrap();
    let dump = temp.path().join("participants.dump");
    let output = std::process::Command::new("pg_dump")
        .args([
            "--dbname",
            &f.admin_url,
            "--schema",
            &f.schema,
            "--format=custom",
            "--file",
        ])
        .arg(&dump)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    f.control
        .batch_execute(&format!("DROP SCHEMA {} CASCADE", f.schema))
        .unwrap();
    let output = std::process::Command::new("pg_restore")
        .args(["--exit-on-error", "--dbname", &f.admin_url])
        .arg(&dump)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(f.snapshot(), before);
    let restored = f.store();
    let current = restored.get(f.owner, f.case, id, f.at).unwrap();
    assert_eq!(current.revision.get(), 3);
    assert_eq!(current.values.display_name(), "After");
    restored
        .change_status(
            f.owner,
            f.case,
            id,
            current.revision,
            DirectoryStatus::Active,
            f.at,
        )
        .unwrap();
}
