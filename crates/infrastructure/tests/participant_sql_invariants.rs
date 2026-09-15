#[allow(dead_code)]
mod participant_database_support;

use application::participants::{
    participant_digest, DirectoryStatus, ParticipantId, ParticipantRevision, ParticipantStore,
    ParticipantValues,
};
use application::ApplicationError;
use infrastructure::RingSha256Hasher;
use participant_database_support::{values, Fixture};
use postgres::error::SqlState;
use postgres::{Client, NoTls};
use std::io::Write;

#[test]
fn sql_rust_and_openssl_agree_on_unicode_and_maximum_participant_canonical_size() {
    let Some(mut f) = Fixture::new() else { return };
    let name = "\u{10000}".repeat(200);
    let role = "\u{10000}".repeat(80);
    let legal = "\u{10000}".repeat(160);
    let v = ParticipantValues::new(
        &name,
        &role,
        Some(&name),
        Some(&legal),
        DirectoryStatus::Archived,
    )
    .unwrap();
    let row=f.admin.query_one("SELECT participant_values_bytes($1,$2,$3,$4,$5),pg_catalog.sha256(participant_values_bytes($1,$2,$3,$4,$5))",
        &[&v.display_name(),&v.procedural_role(),&v.organization(),&v.legal_status(),&v.directory_status().as_str()]).unwrap();
    let canon = v.canonical_bytes();
    assert_eq!(canon.len(), 2584);
    assert_eq!(row.get::<_, Vec<u8>>(0), canon);
    let digest = participant_digest(&RingSha256Hasher::new(), &v);
    assert_eq!(row.get::<_, Vec<u8>>(1), digest.as_bytes());
    let mut command = std::process::Command::new("openssl")
        .args(["dgst", "-sha256", "-binary"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    command.stdin.take().unwrap().write_all(&canon).unwrap();
    let output = command.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, digest.as_bytes());
    for codepoint in (1..=160).chain([
        5760, 8192, 8193, 8194, 8195, 8196, 8197, 8198, 8199, 8200, 8201, 8202, 8203, 8232, 8233,
        8239, 8287, 12288, 0x10000,
    ]) {
        let character = char::from_u32(codepoint).unwrap();
        for name in [
            format!("{character}x"),
            format!("x{character}"),
            format!("x{character}y"),
        ] {
            let expected =
                ParticipantValues::new(&name, "Witness", None, None, DirectoryStatus::Active)
                    .is_ok_and(|v| v.display_name() == name);
            let actual: bool = f
                .admin
                .query_one(
                    "SELECT participant_values_is_canonical($1,'Witness',NULL,NULL,'active')",
                    &[&name],
                )
                .unwrap()
                .get(0);
            assert_eq!(actual, expected, "codepoint={codepoint},name={name:?}");
        }
    }
    for expression in [
        "participant_values_is_canonical(NULL,'Role',NULL,NULL,'active')",
        "participant_values_is_canonical('Name',NULL,NULL,NULL,'active')",
        "participant_values_is_canonical('Name','Role',NULL,NULL,NULL)",
        "participant_values_is_canonical('Name','Role','','','active')",
        "participant_values_is_canonical('Name','Role',NULL,NULL,'ACTIVE')",
        "participant_values_is_canonical(repeat('x',201),'Role',NULL,NULL,'active')",
        "participant_values_is_canonical('Name',repeat('x',81),NULL,NULL,'active')",
        "participant_values_is_canonical('Name','Role',repeat('x',201),NULL,'active')",
        "participant_values_is_canonical('Name','Role',NULL,repeat('x',161),'active')",
    ] {
        assert!(
            !f.admin
                .query_one(&format!("SELECT {expression}"), &[])
                .unwrap()
                .get::<_, bool>(0),
            "accepted {expression}"
        );
    }
}

#[test]
fn sql_rejects_archived_first_revision_gaps_and_rewriting_existing_roots_or_history() {
    let Some(mut f) = Fixture::new() else { return };
    let id = ParticipantId::new();
    let before = f.snapshot();
    let mut runtime = Client::connect(&f.runtime_url, NoTls).unwrap();
    let mut tx = runtime.transaction().unwrap();
    tx.execute(
        "INSERT INTO case_participants(id,case_id) VALUES($1,$2)",
        &[&id.as_uuid(), &f.case.as_uuid()],
    )
    .unwrap();
    let result=tx.execute("INSERT INTO case_participant_revisions(participant_id,revision,display_name,procedural_role,directory_status,values_digest,changed_at,changed_by,changed_by_email)
        VALUES($1,1,'Name','Role','archived',pg_catalog.sha256(participant_values_bytes('Name','Role',NULL,NULL,'archived')),'2025-01-01T00:00:00Z',$2,'actor@example.test')",&[&id.as_uuid(),&f.owner.as_uuid()]);
    assert_eq!(result.unwrap_err().code(), Some(&SqlState::CHECK_VIOLATION));
    tx.rollback().unwrap();
    assert_eq!(f.snapshot(), before);
    let store = f.store();
    store
        .create(f.owner, f.case, id, values("Immutable"), f.at)
        .unwrap();
    let result=runtime.execute("INSERT INTO case_participant_revisions SELECT participant_id,3,display_name,procedural_role,organization,legal_status,directory_status,values_digest,changed_at,changed_by,changed_by_email FROM case_participant_revisions WHERE participant_id=$1",&[&id.as_uuid()]);
    assert_eq!(result.unwrap_err().code(), Some(&SqlState::CHECK_VIOLATION));
    for sql in [
        "UPDATE case_participants SET id=id",
        "DELETE FROM case_participants",
        "TRUNCATE case_participant_revisions",
        "UPDATE case_participant_revisions SET display_name='Changed'",
    ] {
        assert_eq!(
            runtime.batch_execute(sql).unwrap_err().code(),
            Some(&SqlState::INSUFFICIENT_PRIVILEGE)
        );
    }
    for sql in [
        "UPDATE case_participants SET initial_revision=2",
        "UPDATE case_participant_revisions SET changed_by_email='changed@example.test'",
    ] {
        assert_eq!(
            f.admin.batch_execute(sql).unwrap_err().code(),
            Some(&SqlState::CHECK_VIOLATION)
        );
    }
}

#[test]
fn exhausted_revision_does_not_wrap_or_append_an_audit_event() {
    let Some(mut f) = Fixture::new() else { return };
    let store = f.store();
    let id = ParticipantId::new();
    store
        .create(f.owner, f.case, id, values("Maximum"), f.at)
        .unwrap();
    // An administrative sparse fixture reaches the counter boundary without billions of rows.
    f.admin
        .batch_execute(
            "ALTER TABLE case_participant_revisions DISABLE TRIGGER participant_sequence",
        )
        .unwrap();
    f.admin.execute("INSERT INTO case_participant_revisions SELECT participant_id,4294967295,display_name,procedural_role,organization,legal_status,directory_status,values_digest,changed_at,changed_by,changed_by_email FROM case_participant_revisions WHERE participant_id=$1",&[&id.as_uuid()]).unwrap();
    f.admin
        .batch_execute("ALTER TABLE case_participant_revisions ENABLE TRIGGER participant_sequence")
        .unwrap();
    let before = f.snapshot();
    let max = ParticipantRevision::new(u32::MAX).unwrap();
    for result in [
        store.replace(f.owner, f.case, id, max, values("Overflow"), f.at),
        store.change_status(f.owner, f.case, id, max, DirectoryStatus::Archived, f.at),
    ] {
        assert!(matches!(
            result,
            Err(ApplicationError::ParticipantRevisionExhausted)
        ));
    }
    assert_eq!(f.snapshot(), before);
    assert!(matches!(
        infrastructure::PostgresParticipantStore::open(
            &f.runtime_url,
            std::sync::Arc::new(RingSha256Hasher::new())
        ),
        Err(ApplicationError::InvalidConfiguration(_))
    ));
}
