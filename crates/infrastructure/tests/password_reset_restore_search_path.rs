use super::password_reset_backend_support::*;
use super::password_reset_restore_support::*;
use postgres::{Client, NoTls};

struct ShadowSchema {
    connection: Client,
    name: String,
}

impl Drop for ShadowSchema {
    fn drop(&mut self) {
        let _ = self
            .connection
            .batch_execute(&format!("DROP SCHEMA {} CASCADE", self.name));
    }
}

#[test]
fn restoration_never_calls_an_audit_lock_shadow_from_the_connection_search_path() {
    let mut db = fixture();
    let target = account(&mut db, "litigator", true);
    let repository = store(&db);
    issue(&repository, target, 1);
    let request = restore_request(&mut db, uuid::Uuid::new_v4());
    let operation = request.operation_id;
    let before = snapshot(&mut db);
    let mut shadow = ShadowSchema {
        connection: Client::connect(&db.admin_url, NoTls).unwrap(),
        name: format!("reset_restore_shadow_{}", uuid::Uuid::new_v4().simple()),
    };
    shadow
        .connection
        .batch_execute(&format!(
            "CREATE SCHEMA {schema};
         CREATE TABLE {schema}.shadow_calls(marker integer NOT NULL);
         CREATE FUNCTION {schema}.pg_advisory_xact_lock(lock_key bigint) RETURNS void
         LANGUAGE plpgsql SECURITY INVOKER AS $body$
         BEGIN
             INSERT INTO {schema}.shadow_calls(marker) VALUES(1);
             PERFORM pg_catalog.pg_advisory_xact_lock(lock_key);
         END
         $body$;",
            schema = shadow.name,
        ))
        .unwrap();
    let sentinel_before: i64 = shadow
        .connection
        .query_one(
            &format!("SELECT count(*) FROM {}.shadow_calls", shadow.name),
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(sentinel_before, 0);

    let mut url = reqwest::Url::parse(&db.admin_url).unwrap();
    let parameters: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(key, _)| key != "options")
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    url.set_query(None);
    url.query_pairs_mut().extend_pairs(parameters).append_pair(
        "options",
        &format!("-csearch_path={},{},pg_catalog", shadow.name, db.schema),
    );
    let mut connection = Client::connect(url.as_str(), NoTls).unwrap();
    let resolved: String = connection
        .query_one(
            "SELECT n.nspname::text FROM pg_catalog.pg_proc p
         JOIN pg_catalog.pg_namespace n ON n.oid=p.pronamespace
         WHERE p.oid='pg_advisory_xact_lock(bigint)'::pg_catalog.regprocedure",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(
        resolved, shadow.name,
        "fixture did not make the shadow visible"
    );
    drop(connection);

    let result = invalidate_restored_password_resets(url.as_str(), request).unwrap();
    let after = snapshot(&mut db);
    let sentinel_after: i64 = shadow
        .connection
        .query_one(
            &format!("SELECT count(*) FROM {}.shadow_calls", shadow.name),
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(
        sentinel_after, sentinel_before,
        "restoration executed a function from the caller search-path prefix"
    );
    assert!(result.applied);
    assert_eq!(result.invalidated, 1);
    for table in ["users", "members"] {
        assert!(
            after[table] == before[table],
            "restoration changed private identity state"
        );
    }
    assert!(repository.inspect(digest(1)).unwrap().is_none());
    assert_restore_receipt(&mut db, operation, &result);
}
