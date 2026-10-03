use crate::password_reset_backend_support::Fixture;
use crate::password_reset_restore_support::{
    PasswordResetRestoreRequest, PasswordResetRestoreResult,
};
use serde_json::{json, Value};
use std::process::{Command, Output};

pub const INVALIDATE: &str = "invalidate-restored-password-resets";
pub const PRIVATE_SENTINEL: &str = "restore-cli-private-fixture";

pub fn invoke(url: Option<&str>, arguments: &[String]) -> Output {
    let directory = tempfile::tempdir().unwrap();
    // Prevent dotenv from searching a developer's parent directory.
    std::fs::write(directory.path().join(".env"), b"").unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_despacho-cli"));
    command
        .current_dir(directory.path())
        .env_clear()
        .env("RUST_LOG", "off");
    if let Some(url) = url {
        command.env("DATABASE_URL", url);
    }
    let output = command.args(arguments).output().expect("the CLI starts");
    for bytes in [&output.stdout, &output.stderr] {
        let text = String::from_utf8_lossy(bytes);
        assert!(
            !text.contains(PRIVATE_SENTINEL),
            "CLI exposed private input"
        );
        assert!(
            !text.contains("runtime-test-only"),
            "CLI exposed fixture credentials"
        );
        for private in ["original-hash", "confirmed-before-backup", "@example.test"] {
            assert!(!text.contains(private), "CLI exposed private fixture data");
        }
        if let Some(url) = url {
            assert!(!text.contains(url), "CLI exposed its database URL");
        }
    }
    output
}

pub fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

pub fn restore_args(request: &PasswordResetRestoreRequest) -> Vec<String> {
    let mut values = args(&["--json", "database", INVALIDATE, "--operation-id"]);
    values.extend([
        request.operation_id.to_string(),
        "--expected-database".into(),
        request.expected_database.clone(),
        "--expected-schema".into(),
        request.expected_schema.clone(),
    ]);
    if let Some(head) = request.expected_head {
        values.extend([
            "--expected-audit-sequence".into(),
            head.sequence.to_string(),
            "--expected-audit-head".into(),
            head.chain.to_hex(),
        ]);
    } else {
        values.push("--expected-empty-audit".into());
    }
    values
}

pub fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "CLI failed; private output withheld"
    );
    assert!(
        output.stderr.is_empty(),
        "successful CLI emitted diagnostics"
    );
    serde_json::from_slice(&output.stdout).expect("CLI must emit exactly one JSON value")
}

pub fn rejected(output: Output) {
    assert!(
        !output.status.success(),
        "invalid operation unexpectedly succeeded"
    );
    assert!(
        output.stdout.is_empty(),
        "rejected operation emitted a receipt"
    );
    assert!(
        !output.stderr.is_empty(),
        "rejected operation needs a neutral diagnostic"
    );
}

pub fn expected_receipt(id: uuid::Uuid, result: PasswordResetRestoreResult) -> Value {
    json!({
        "operation_id": id.to_string(),
        "applied": result.applied,
        "invalidated": result.invalidated,
        "audit_sequence": result.audit_sequence,
        "audit_head": result.audit_head.to_hex(),
    })
}

pub fn read_only_url(url: &str) -> String {
    let mut parsed = reqwest::Url::parse(url).unwrap();
    let parameters: Vec<_> = parsed
        .query_pairs()
        .map(|(key, value)| {
            if key == "options" {
                (
                    key.into_owned(),
                    format!("{value} -cdefault_transaction_read_only=on"),
                )
            } else {
                (key.into_owned(), value.into_owned())
            }
        })
        .collect();
    parsed.query_pairs_mut().clear().extend_pairs(parameters);
    // PostgreSQL URI options use percent escapes rather than form-encoded spaces.
    parsed.to_string().replace('+', "%20")
}

pub fn state(db: &mut Fixture) -> Value {
    let catalog: Value = db
        .admin
        .query_one(
            "SELECT jsonb_build_object(
         'namespace',(SELECT to_jsonb(n) FROM pg_namespace n WHERE n.nspname=$1),
         'relations',(SELECT jsonb_agg(jsonb_build_array(c.oid,c.relname,c.relkind,
             c.relowner,c.relacl,c.reloptions,c.relrowsecurity,c.relforcerowsecurity)
             ORDER BY c.oid) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
             WHERE n.nspname=$1),
         'functions',(SELECT jsonb_agg(to_jsonb(p) ORDER BY p.oid) FROM pg_proc p
             JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname=$1),
         'constraints',(SELECT jsonb_agg(to_jsonb(c) ORDER BY c.oid) FROM pg_constraint c
             JOIN pg_namespace n ON n.oid=c.connamespace WHERE n.nspname=$1),
         'columns',(SELECT jsonb_agg(to_jsonb(a) ORDER BY a.attrelid,a.attnum)
             FROM pg_attribute a JOIN pg_class c ON c.oid=a.attrelid
             JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname=$1),
         'triggers',(SELECT jsonb_agg(to_jsonb(t) ORDER BY t.oid) FROM pg_trigger t
             JOIN pg_class c ON c.oid=t.tgrelid JOIN pg_namespace n ON n.oid=c.relnamespace
             WHERE n.nspname=$1),
         'role',(SELECT to_jsonb(r) FROM pg_roles r WHERE r.rolname=$2))",
            &[&db.schema, &db.role],
        )
        .unwrap()
        .get(0);
    let tables = db
        .admin
        .query(
            "SELECT pg_catalog.format('%I.%I',n.nspname,c.relname) FROM pg_class c
         JOIN pg_namespace n ON n.oid=c.relnamespace
         WHERE n.nspname=$1 AND c.relkind IN ('r','p') ORDER BY c.relname",
            &[&db.schema],
        )
        .unwrap();
    let mut rows = serde_json::Map::new();
    for table in tables {
        let name: String = table.get(0);
        let value: Value = db.admin.query_one(
            &format!("SELECT coalesce(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text),'[]'::jsonb) FROM {name} t"),
            &[],
        ).unwrap().get(0);
        rows.insert(name, value);
    }
    json!({"catalog":catalog,"rows":rows})
}

pub fn unchanged(db: &mut Fixture, before: &Value) {
    assert!(
        state(db) == *before,
        "private database state or catalog changed"
    );
}
