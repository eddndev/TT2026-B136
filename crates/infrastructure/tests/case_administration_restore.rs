mod case_administration_support;
use application::cases::*;
use case_administration_support::Fixture;
use domain::cases::{CaseId, CaseMetadata};

#[test]
fn dump_restore_preserves_baseline_full_history_stage_provenance_and_runtime_mutations() {
    let Some(mut f) = Fixture::new() else { return };
    let store = f.store();
    let id = CaseId::new();
    let profile = PenalCaseProfile::new(
        "NUC",
        "Office",
        "FOLDER",
        "Court",
        &["Reported"],
        Some("First\nSecond"),
        None,
    )
    .unwrap();
    let first = store
        .register_penal(
            f.owner,
            id,
            PenalCaseCreation::new(CaseMetadata::new("Original", "REF").unwrap(), profile),
            f.at,
        )
        .unwrap();
    let stage = first.initial_stage.unwrap();
    store
        .change_administrative_status(
            f.owner,
            id,
            CaseRevisionExpectation::new(1),
            CaseAdministrativeStatus::Closed,
            f.at,
        )
        .unwrap();
    drop(store);
    let before = f.snapshot();
    f.migrate();
    assert_eq!(f.snapshot(), before);
    let temp = tempfile::tempdir().unwrap();
    let dump = temp.path().join("cases.dump");
    let result = std::process::Command::new("pg_dump")
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
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    f.control
        .batch_execute(&format!("DROP SCHEMA {} CASCADE", f.schema))
        .unwrap();
    let result = std::process::Command::new("pg_restore")
        .args(["--exit-on-error", "--dbname", &f.admin_url])
        .arg(&dump)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(f.snapshot(), before);
    let store = f.store();
    assert_eq!(
        store
            .get_administration(f.owner, id, f.at)
            .unwrap()
            .initial_stage,
        Some(stage)
    );
    assert!(matches!(
        store
            .get_administration(f.owner, f.case, f.at)
            .unwrap()
            .administration,
        CurrentCaseAdministration::Unrevised(_)
    ));
    store
        .change_administrative_status(
            f.owner,
            id,
            CaseRevisionExpectation::new(2),
            CaseAdministrativeStatus::Active,
            f.at,
        )
        .unwrap();
}
#[test]
fn revision_sequence_and_initial_stage_guards_are_schema_bound_with_empty_search_path() {
    let Some(f) = Fixture::new() else { return };
    let id = CaseId::new();
    let mut runtime = f.runtime();
    runtime.batch_execute("SET search_path='' ").unwrap();
    let mut tx = runtime.transaction().unwrap();
    tx.execute(
        &format!(
            "INSERT INTO {}.cases(id,title,reference,created_by) VALUES($1,'New','REF',$2)",
            f.schema
        ),
        &[&id.as_uuid(), &f.owner.as_uuid()],
    )
    .unwrap();
    tx.execute(&format!("INSERT INTO {0}.case_administration_revisions(case_id,revision,title,reference,administrative_status,nuc,nuc_authority,judicial_case_number,judicial_authority,offenses,values_digest,changed_at,changed_by,changed_by_email) VALUES($1,1,'New','REF','active','NUC','Office','FOLDER','Court',ARRAY['Reported'],sha256({0}.case_administration_bytes('active','New','REF','NUC','Office','FOLDER','Court',ARRAY['Reported'],NULL,NULL)),'2025-01-01T00:00:00Z',$2,'owner@example.test')",f.schema), &[&id.as_uuid(),&f.owner.as_uuid()]).unwrap();
    tx.execute(&format!("INSERT INTO {}.case_initial_stage_registrations(case_id,stage) VALUES($1,'investigation')",f.schema), &[&id.as_uuid()]).unwrap();
    tx.commit().unwrap();
    f.store().get_administration(f.owner, id, f.at).unwrap();
}
