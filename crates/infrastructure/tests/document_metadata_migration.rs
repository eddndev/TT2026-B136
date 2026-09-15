#[allow(dead_code)]
mod version_database_support;

use application::documents::{metadata_digest, DocumentMetadata};
use infrastructure::PostgresCaseDocumentStore;
use std::io::Write;
use version_database_support::Database;

#[test]
fn sql_metadata_canon_matches_fixed_sha256_vectors_without_extensions() {
    let Some(mut db) = Database::old_schema() else {
        return;
    };
    PostgresCaseDocumentStore::connect(&db.url).unwrap();
    for (kind, class, tags, expected, digest) in [
        (
            None,
            None,
            vec![],
            "444d45544131000000000000",
            "adbad13daff0fc70b3309e1e58a20aecadc00077dac6f2a04349c4001e8443bb",
        ),
        (
            Some("Escrito"),
            None,
            vec!["a", "z"],
            "444d4554413101000000074573637269746f00000000020000000161000000017a",
            "bf6464a7b8f3d99e58153745b1b524ba8b8f03baed73100d0419bb39cc011156",
        ),
        (
            Some("Tipo"),
            Some("Civil"),
            vec!["a,b", "\u{e1}"],
            "444d4554413101000000045469706f0100000005436976696c0000000200000003612c6200000002c3a1",
            "b18ac2091c70aaa484cfdd8e060919c6692a606bb48224d27222c3af682817c1",
        ),
    ] {
        let row = db.client.query_one(
            "SELECT encode(document_metadata_bytes($1,$2,$3),'hex'), encode(pg_catalog.sha256(document_metadata_bytes($1,$2,$3)),'hex'), document_metadata_is_canonical($1,$2,$3),document_metadata_bytes($1,$2,$3)",
            &[&kind, &class, &tags],
        ).unwrap();
        assert_eq!(row.get::<_, String>(0), expected);
        assert_eq!(row.get::<_, String>(1), digest);
        assert!(row.get::<_, bool>(2));
        let values = DocumentMetadata::new(
            kind,
            class,
            &tags.iter().map(|tag| tag.to_string()).collect::<Vec<_>>(),
        )
        .unwrap();
        let canon = values.canonical_bytes();
        assert_eq!(row.get::<_, Vec<u8>>(3), canon);
        let hash = metadata_digest(&infrastructure::RingSha256Hasher::new(), &values);
        assert_eq!(hash.to_hex(), digest);
        let mut openssl = std::process::Command::new("openssl")
            .args(["dgst", "-sha256", "-binary"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        openssl.stdin.take().unwrap().write_all(&canon).unwrap();
        let output = openssl.wait_with_output().unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, hash.as_bytes());
    }
    assert!(!db
        .client
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname='pgcrypto')",
            &[]
        )
        .unwrap()
        .get::<_, bool>(0));
}

#[test]
fn migration_preserves_legacy_versions_and_does_not_invent_metadata_revisions() {
    let Some(mut db) = Database::old_schema() else {
        return;
    };
    let case = db.seed_case();
    let id = uuid::Uuid::new_v4();
    db.client
        .execute(
            "INSERT INTO documents VALUES($1,$2,7,'legacy.txt',$3,$4,$5)",
            &[
                &id,
                &case,
                &vec![3_u8; 32],
                &vec![7_u8; 80],
                &serde_json::json!({"captured":"unchanged"}),
            ],
        )
        .unwrap();
    let before = db.snapshot();
    PostgresCaseDocumentStore::connect(&db.url).unwrap();
    assert_eq!(db.snapshot(), before);
    assert_eq!(
        db.client
            .query_one("SELECT COUNT(*) FROM document_metadata_revisions", &[])
            .unwrap()
            .get::<_, i64>(0),
        0
    );
    db.client.execute("INSERT INTO document_metadata_revisions(document_id,metadata_revision,document_type,tags,metadata_digest,changed_at,changed_by,changed_by_email) SELECT $1,1,'Type',ARRAY['a,b'],pg_catalog.sha256(document_metadata_bytes('Type',NULL,ARRAY['a,b'])),'2025-01-01T00:00:00.123456789Z',created_by,'historical@example.test' FROM cases WHERE id=$2", &[&id,&case]).unwrap();
    let metadata: serde_json::Value = db
        .client
        .query_one("SELECT to_jsonb(m) FROM document_metadata_revisions m", &[])
        .unwrap()
        .get(0);
    PostgresCaseDocumentStore::connect(&db.url).unwrap();
    assert_eq!(db.snapshot(), before);
    assert_eq!(
        db.client
            .query_one("SELECT to_jsonb(m) FROM document_metadata_revisions m", &[])
            .unwrap()
            .get::<_, serde_json::Value>(0),
        metadata
    );
}

#[test]
fn sql_metadata_rejects_noncanonical_values_and_nonstandard_arrays() {
    let Some(mut db) = Database::old_schema() else {
        return;
    };
    PostgresCaseDocumentStore::connect(&db.url).unwrap();
    for expression in [
        "document_metadata_is_canonical('',NULL,ARRAY[]::text[])",
        "document_metadata_is_canonical(' x',NULL,ARRAY[]::text[])",
        "document_metadata_is_canonical(chr(160)||'x',NULL,ARRAY[]::text[])",
        "document_metadata_is_canonical('x'||chr(8239),NULL,ARRAY[]::text[])",
        "document_metadata_is_canonical(chr(9)||'x',NULL,ARRAY[]::text[])",
        "document_metadata_is_canonical(chr(159),NULL,ARRAY[]::text[])",
        "document_metadata_is_canonical(repeat('x',81),NULL,ARRAY[]::text[])",
        "document_metadata_is_canonical(NULL,NULL,ARRAY['z','a'])",
        "document_metadata_is_canonical(NULL,NULL,ARRAY['a','a'])",
        "document_metadata_is_canonical(NULL,NULL,ARRAY[''])",
        "document_metadata_is_canonical(NULL,NULL,ARRAY[NULL]::text[])",
        "document_metadata_is_canonical(NULL,NULL,array_fill('a'::text,ARRAY[21]))",
        "document_metadata_is_canonical(NULL,NULL,ARRAY[['a','b']])",
        "document_metadata_is_canonical(NULL,NULL,'[0:0]={a}'::text[])",
    ] {
        assert!(
            !db.client
                .query_one(&format!("SELECT {expression}"), &[])
                .unwrap()
                .get::<_, bool>(0),
            "accepted {expression}"
        );
    }
}

#[test]
fn sql_and_rust_agree_on_unicode_boundaries_and_maximum_canonical_size() {
    let Some(mut db) = Database::old_schema() else {
        return;
    };
    PostgresCaseDocumentStore::connect(&db.url).unwrap();
    let kind = "\u{10000}".repeat(80);
    let tags = (0..20)
        .map(|i| char::from_u32(0x10000 + i).unwrap().to_string().repeat(40))
        .collect::<Vec<_>>();
    let values = DocumentMetadata::new(Some(&kind), Some(&kind), &tags).unwrap();
    assert_eq!(values.canonical_bytes().len(), 3940);
    let actual: Vec<u8> = db
        .client
        .query_one(
            "SELECT document_metadata_bytes($1,$2,$3)",
            &[
                &values.document_type(),
                &values.classification(),
                &values.tags(),
            ],
        )
        .unwrap()
        .get(0);
    assert_eq!(actual, values.canonical_bytes());
    for codepoint in (1..=160).chain([
        5760, 8192, 8193, 8194, 8195, 8196, 8197, 8198, 8199, 8200, 8201, 8202, 8203, 8232, 8233,
        8239, 8287, 12288, 0x10000,
    ]) {
        let character = char::from_u32(codepoint).unwrap();
        for value in [
            format!("{character}x"),
            format!("x{character}"),
            format!("x{character}y"),
        ] {
            let expected = DocumentMetadata::new(Some(&value), None, &[])
                .is_ok_and(|m| m.document_type() == Some(value.as_str()));
            let actual: bool = db
                .client
                .query_one(
                    "SELECT document_metadata_is_canonical($1,NULL,ARRAY[]::text[])",
                    &[&value],
                )
                .unwrap()
                .get(0);
            assert_eq!(actual, expected, "codepoint={codepoint},value={value:?}");
        }
    }
}
