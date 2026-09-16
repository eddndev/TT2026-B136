#[allow(dead_code)]
mod participant_database_support;

use participant_database_support::Fixture;
use postgres::error::SqlState;

#[test]
fn participant_sql_canon_matches_fixed_vectors_without_pgcrypto() {
    let Some(mut f) = Fixture::new() else { return };
    for (name, role, organization, status, archived, bytes, digest) in [
        ("Ana", "Witness", None, None, "active", "504152543100000003416e61000000075769746e657373000000", "fc4e619a010411b046c79cbd60bfd6292c9948cbef2d5caea6ae2c17a88263ad"),
        ("Jos\u{e9}", "Defensor", Some("Despacho"), Some("Registrado"), "archived", "5041525431000000054a6f73c3a900000008446566656e736f720100000008446573706163686f010000000a5265676973747261646f01", "08d0781132340e01ce569f791cc7da6ba2f124602d1f83854c0f36e59920165d"),
        ("A\u{10000}", "Rol", Some("a,b"), None, "active", "50415254310000000541f090808000000003526f6c0100000003612c620000", "4bb9c6ac16049b2791e98c1e542d738cd86c891d45f25b48f6a41762e493d6d6"),
    ] {
        let actual = f.admin.query_one("SELECT encode(participant_values_bytes($1,$2,$3,$4,$5),'hex'), encode(pg_catalog.sha256(participant_values_bytes($1,$2,$3,$4,$5)),'hex')", &[&name,&role,&organization,&status,&archived]).unwrap();
        assert_eq!(actual.get::<_,String>(0), bytes);
        assert_eq!(actual.get::<_,String>(1), digest);
    }
    assert!(!f
        .admin
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname='pgcrypto')",
            &[]
        )
        .unwrap()
        .get::<_, bool>(0));
}

#[test]
fn participant_roots_require_an_active_first_revision_at_commit() {
    let Some(mut f) = Fixture::new() else { return };
    let id = uuid::Uuid::new_v4();
    let result = f.admin.execute(
        "INSERT INTO case_participants(id,case_id) VALUES($1,$2)",
        &[&id, &f.case.as_uuid()],
    );
    let failure = result.unwrap_err();
    assert_eq!(failure.code(), Some(&SqlState::CHECK_VIOLATION));
    assert_eq!(
        failure.as_db_error().unwrap().message(),
        "participant requires exactly one initial revision"
    );
    assert_eq!(
        f.admin
            .query_one("SELECT COUNT(*) FROM case_participants", &[])
            .unwrap()
            .get::<_, i64>(0),
        0
    );
}
