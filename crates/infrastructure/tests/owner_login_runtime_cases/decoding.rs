use base64::{engine::general_purpose::STANDARD, Engine};
use redis::Commands;
use serde_json::{json, Value};

use super::{material, support::*};

fn rejects(db: &mut Fixture, raw: &str, deadline: i64) {
    let (token, key) = db.token();
    db.string(&key, raw, deadline);
    let store = db.store(policy(20, 10));
    assert!(
        store.take(&token).unwrap().is_none(),
        "invalid capture became a proof"
    );
    assert!(
        !db.connection.exists::<_, bool>(&key).unwrap(),
        "invalid capture was not consumed"
    );
    assert!(store.take(&token).unwrap().is_none());
}

#[test]
fn every_capture_object_rejects_unknown_fields_and_duplicate_fields() {
    let mut db = Fixture::new();
    let value = material::capture(db.now() / 1000, 90);
    let wire = material::wire(&value);
    let deadline = value.statement.expires_at_unix_seconds() * 1000;
    for path in [
        "",
        "/context",
        "/context/account",
        "/context/account/principal",
        "/context/certificate",
        "/context/certificate/summary",
        "/context/trust",
        "/context/trust/inspection",
        "/context/trust/published_at",
    ] {
        let mut changed = wire.clone();
        changed
            .pointer_mut(path)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), json!(true));
        rejects(&mut db, &changed.to_string(), deadline);
        let object = wire.pointer(path).unwrap().as_object().unwrap();
        let (field, original) = object.iter().next().unwrap();
        let encoded_object = serde_json::to_string(object).unwrap();
        let duplicate = format!(
            "{{{}:{},{}",
            serde_json::to_string(field).unwrap(),
            original,
            &encoded_object[1..]
        );
        let raw = wire.to_string().replacen(&encoded_object, &duplicate, 1);
        assert_ne!(
            raw,
            wire.to_string(),
            "duplicate-field fixture was not inserted"
        );
        rejects(&mut db, &raw, deadline);
    }
}

#[test]
fn changed_statement_material_purpose_account_or_private_dto_encoding_cannot_be_claimed() {
    let mut db = Fixture::new();
    let value = material::capture(db.now() / 1000, 90);
    let original = material::wire(&value);
    let deadline = value.statement.expires_at_unix_seconds() * 1000;
    let replacements: Vec<(&str, Value)> = vec![
        ("/version", json!(2)),
        ("/version", json!("1")),
        ("/context", json!([])),
        ("/context/account/active", json!(false)),
        ("/context/account/principal/role", json!("litigator")),
        ("/context/account/principal/id", json!(Uuid::new_v4())),
        (
            "/context/account/auth_generation",
            json!(9_007_199_254_740_992_u64),
        ),
        ("/context/account/revision", json!(0)),
        (
            "/context/account/auth_generation",
            json!("9007199254740993"),
        ),
        ("/context/binding_id", json!(Uuid::new_v4())),
        ("/context/trust/deployment_id", json!(Uuid::nil())),
        ("/context/trust/revision", json!(8)),
        (
            "/context/trust/inspection/crl_der",
            json!(STANDARD.encode(b"other-crl")),
        ),
        (
            "/context/trust/inspection/root_der",
            json!(STANDARD.encode(b"other-root")),
        ),
        (
            "/context/certificate/der",
            json!(STANDARD.encode(b"other-leaf")),
        ),
        ("/context/certificate/der", json!([1, 2, 3])),
        (
            "/context/trust/inspection/crl_digest",
            json!(value
                .context
                .trust
                .inspection
                .crl_digest
                .to_hex()
                .to_uppercase()),
        ),
        (
            "/context/binding_id",
            json!(value.context.binding_id.simple().to_string()),
        ),
        ("/context/certificate/fingerprint", json!("00".repeat(32))),
        (
            "/context/trust/published_at/nanoseconds",
            json!(1_000_000_000),
        ),
        ("/statement", json!(STANDARD.encode([0_u8; 150]))),
        (
            "/statement",
            json!(format!("{}\n", original["statement"].as_str().unwrap())),
        ),
    ];
    for (path, replacement) in replacements {
        let mut changed = original.clone();
        *changed.pointer_mut(path).unwrap() = replacement;
        rejects(&mut db, &changed.to_string(), deadline);
    }
    for offset in [0, 8, 9, 10, 26, 58, 62, 78, 86, 102] {
        let mut bytes = value.statement.canonical_bytes();
        bytes[offset] ^= 1;
        let mut changed = original.clone();
        changed["statement"] = json!(STANDARD.encode(bytes));
        rejects(&mut db, &changed.to_string(), deadline);
    }
    for (issued, expires) in [
        (
            value.statement.issued_at_unix_seconds(),
            value.statement.expires_at_unix_seconds() + 1,
        ),
        (
            value.statement.expires_at_unix_seconds(),
            value.statement.expires_at_unix_seconds(),
        ),
        (
            value.statement.issued_at_unix_seconds(),
            value.statement.issued_at_unix_seconds() + 301,
        ),
    ] {
        let mut bytes = value.statement.canonical_bytes();
        bytes[166..174].copy_from_slice(&issued.to_be_bytes());
        bytes[174..182].copy_from_slice(&expires.to_be_bytes());
        let mut changed = original.clone();
        changed["statement"] = json!(STANDARD.encode(bytes));
        rejects(&mut db, &changed.to_string(), deadline);
    }
}

#[test]
fn corrupt_json_oversized_body_and_non_string_records_are_consumed() {
    let mut db = Fixture::new();
    let deadline = db.now() + 60_000;
    for raw in ["null", "[]", "{", "{\"version\":1}"] {
        rejects(&mut db, raw, deadline);
    }
    rejects(&mut db, &"x".repeat(4 * 1024 * 1024 + 1), deadline);
    for list in [true, false] {
        let (token, key) = db.token();
        if list {
            db.connection
                .lpush::<_, _, ()>(&key, "public-corrupt-capture")
                .unwrap();
        } else {
            db.connection
                .hset::<_, _, _, ()>(&key, "version", 1)
                .unwrap();
        }
        db.expire(&key, deadline);
        assert!(db.store(policy(20, 10)).take(&token).unwrap().is_none());
        assert!(!db.connection.exists::<_, bool>(&key).unwrap());
    }
}
