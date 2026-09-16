#[allow(dead_code)]
mod credential_trust_database_support;

use std::sync::Arc;

use infrastructure::{
    initialize_database, PostgresCaseRepository, PostgresCredentialTrustStore, RingSha256Hasher,
};
use postgres::{Client, NoTls};

use credential_trust_database_support::{materials, Clock, Database};

#[test]
fn runtime_can_read_trust_but_cannot_publish_mutate_or_own_it() {
    let Some(mut db) = Database::new() else {
        return;
    };
    let runtime_url = db.role("");
    initialize_database(&db.url, &db.roles[0]).unwrap();
    PostgresCaseRepository::open(&runtime_url, Arc::new(RingSha256Hasher)).unwrap();
    let mut runtime = Client::connect(&runtime_url, NoTls).unwrap();
    runtime
        .query("SELECT * FROM participant_credential_authority", &[])
        .unwrap();
    runtime
        .query("SELECT * FROM participant_credential_trust_revisions", &[])
        .unwrap();
    assert!(PostgresCredentialTrustStore::open(&runtime_url, Clock::new(materials().at)).is_err());
    for statement in [
        "INSERT INTO participant_credential_authority DEFAULT VALUES",
        "INSERT INTO participant_credential_trust_revisions DEFAULT VALUES",
        "UPDATE participant_credential_authority SET root_der=root_der",
        "UPDATE participant_credential_trust_revisions SET crl_number=crl_number",
        "DELETE FROM participant_credential_authority",
        "DELETE FROM participant_credential_trust_revisions",
        "TRUNCATE participant_credential_authority,participant_credential_trust_revisions",
        "ALTER TABLE participant_credential_trust_revisions DISABLE TRIGGER credential_trust_sequence",
        "SELECT enforce_credential_trust_sequence()",
    ] {
        assert_eq!(runtime.batch_execute(statement).unwrap_err().code().unwrap().code(), "42501", "{statement}");
    }
}

#[test]
fn runtime_startup_rejects_direct_inherited_column_and_function_privileges() {
    for grant in [
        "GRANT INSERT ON participant_credential_trust_revisions TO {role}",
        "GRANT UPDATE(crl_number) ON participant_credential_trust_revisions TO {role}",
        "GRANT DELETE ON participant_credential_authority TO {role}",
        "GRANT TRUNCATE ON participant_credential_trust_revisions TO {role}",
        "GRANT EXECUTE ON FUNCTION enforce_credential_trust_sequence() TO {role}",
        "ALTER FUNCTION preserve_credential_trust_history() OWNER TO {role}",
        "GRANT INSERT ON participant_credential_authority TO PUBLIC",
    ] {
        let Some(mut db) = Database::new() else {
            return;
        };
        let runtime_url = db.role("");
        initialize_database(&db.url, &db.roles[0]).unwrap();
        db.client
            .batch_execute(&grant.replace("{role}", &db.roles[0]))
            .unwrap();
        assert!(
            PostgresCaseRepository::open(&runtime_url, Arc::new(RingSha256Hasher)).is_err(),
            "{grant}"
        );
    }
    let Some(mut db) = Database::new() else {
        return;
    };
    let runtime_url = db.role("");
    initialize_database(&db.url, &db.roles[0]).unwrap();
    db.role("GRANT INSERT ON participant_credential_authority TO {role}");
    db.client
        .batch_execute(&format!("GRANT {} TO {}", db.roles[1], db.roles[0]))
        .unwrap();
    assert!(PostgresCaseRepository::open(&runtime_url, Arc::new(RingSha256Hasher)).is_err());
}
