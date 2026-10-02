use super::password_reset_backend_support::*;

#[test]
fn runtime_cannot_read_reset_digests_or_rewrite_credentials_despite_forged_settings() {
    let mut db = fixture();
    account(&mut db, "litigator", true);
    let _repository = store(&db);
    let before = snapshot(&mut db);
    let mut runtime = db.runtime();
    runtime
        .batch_execute("SET qadra.reset='true'; SET application_name='password-reset'")
        .unwrap();
    for sql in [
        "SELECT * FROM password_reset_capabilities WHERE false",
        "INSERT INTO password_reset_capabilities(id) VALUES('00000000-0000-0000-0000-000000000001')",
        "UPDATE password_reset_capabilities SET cancelled_at=clock_timestamp() WHERE false",
        "DELETE FROM password_reset_capabilities WHERE false",
        "TRUNCATE password_reset_capabilities",
        "UPDATE users SET password_hash='bypass',revision=revision+1,auth_generation=auth_generation+1 WHERE false",
        "UPDATE users SET protected_totp_secret='\\x00' WHERE false",
    ] {
        let error = runtime.batch_execute(sql).expect_err("runtime must not have direct credential authority");
        assert_eq!(error.code(), Some(&postgres::error::SqlState::INSUFFICIENT_PRIVILEGE), "wrong rejection for {sql}: {error}");
    }
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn expected_receipt_foreign_key_is_accepted_but_an_unrelated_foreign_key_is_rejected() {
    let mut db = fixture();
    let _administrative = PostgresPasswordResetRepository::connect(&db.admin_url).unwrap();
    store(&db);
    let internal: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM pg_trigger t JOIN pg_constraint c ON c.oid=t.tgconstraint
         WHERE t.tgrelid='audit_events'::regclass AND t.tgisinternal
         AND c.conrelid='password_reset_capabilities'::regclass AND c.contype='f'",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(internal, 2);
    db.admin.batch_execute("CREATE TABLE unexpected_audit_reference(sequence bigint REFERENCES audit_events(sequence))").unwrap();
    assert!(PostgresPasswordResetRepository::open(&db.runtime_url).is_err());
}

#[test]
fn an_unrelated_external_audit_trigger_is_not_whitelisted_by_the_reset_receipt() {
    let mut db = fixture();
    store(&db);
    db.admin.batch_execute(
        "CREATE FUNCTION unexpected_audit_trigger() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RETURN NEW; END $$;
         CREATE TRIGGER unexpected_audit_trigger BEFORE INSERT ON audit_events
         FOR EACH ROW EXECUTE FUNCTION unexpected_audit_trigger()",
    ).unwrap();
    assert!(PostgresPasswordResetRepository::open(&db.runtime_url).is_err());
}

#[test]
fn startup_rejects_exposed_reset_authority_and_altered_function_configuration() {
    for change in [
        "public",
        "grant_option",
        "table_read",
        "search_path",
        "invoker",
        "overload",
        "body",
        "member_guard",
    ] {
        let mut db = fixture();
        store(&db);
        let sql = match change {
            "public" => "GRANT EXECUTE ON FUNCTION password_reset_consume(uuid,bytea,uuid,bigint,text) TO PUBLIC".into(),
            "grant_option" => format!("GRANT EXECUTE ON FUNCTION password_reset_cancel(uuid,bytea) TO {} WITH GRANT OPTION", db.role),
            "table_read" => format!("GRANT SELECT ON password_reset_capabilities TO {}", db.role),
            "search_path" => "ALTER FUNCTION password_reset_consume(uuid,bytea,uuid,bigint,text) SET search_path TO public".into(),
            "invoker" => "ALTER FUNCTION password_reset_consume(uuid,bytea,uuid,bigint,text) SECURITY INVOKER".into(),
            "overload" => "CREATE FUNCTION password_reset_consume(text) RETURNS boolean LANGUAGE sql AS $$ SELECT true $$".into(),
            "body" => "CREATE OR REPLACE FUNCTION password_reset_inventory_valid() RETURNS boolean LANGUAGE sql SECURITY DEFINER SET search_path=pg_catalog AS $$ SELECT true $$".into(),
            "member_guard" => "CREATE OR REPLACE FUNCTION guard_member_access() RETURNS trigger LANGUAGE plpgsql SET search_path=pg_catalog AS $$ BEGIN RETURN NEW; END $$".into(),
            _ => unreachable!(),
        };
        db.admin.batch_execute(&sql).unwrap();
        assert!(
            PostgresPasswordResetRepository::open(&db.runtime_url).is_err(),
            "accepted {change}"
        );
    }
}
