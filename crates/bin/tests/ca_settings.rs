#[allow(dead_code)]
#[path = "../src/ca/config.rs"]
mod config;

use std::os::unix::fs::{symlink, PermissionsExt};

fn settings() -> serde_json::Value {
    serde_json::json!({
        "bind": "127.0.0.1:18089", "database_url": "postgresql://ca:private@localhost/qadra",
        "database_name": "qadra", "database_role": "ca", "redis_url": "redis://:private@localhost/",
        "kek_base64": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
        "operator_id": "20000000-0000-4000-8000-000000000001", "operator_email": "ca@example.test",
        "session_ttl_seconds": 3600, "idle_ttl_seconds": 900
    })
}

fn save(path: &std::path::Path, value: &serde_json::Value) {
    std::fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
}

#[test]
fn ca_settings_are_private_bounded_and_never_accept_symlink_or_unknown_fields() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("ca.json");
    save(&path, &settings());
    let accepted = config::load(&path).unwrap();
    assert_eq!(accepted.operator().unwrap().email, "ca@example.test");
    assert_eq!(accepted.policy().unwrap().idle_ttl_seconds(), Some(900));
    assert_eq!(accepted.kek().unwrap().len(), 32);
    let link = temp.path().join("link");
    symlink(&path, &link).unwrap();
    assert!(config::load(&link).is_err());
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(config::load(&path).is_err());
    let mut extra = settings();
    extra["office_owner_access"] = true.into();
    save(&path, &extra);
    assert!(config::load(&path).is_err());
    std::fs::write(&path, vec![b' '; 65537]).unwrap();
    assert!(config::load(&path).is_err());
}

#[test]
fn invalid_operator_lifetimes_keys_and_public_bind_fail_before_opening_dependencies() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("ca.json");
    for (field, value) in [
        ("bind", serde_json::json!("0.0.0.0:18089")),
        ("bind", serde_json::json!("127.0.0.1:0")),
        ("operator_id", serde_json::json!(uuid::Uuid::nil())),
        ("operator_email", serde_json::json!("not-email")),
        ("session_ttl_seconds", serde_json::json!(0)),
        ("idle_ttl_seconds", serde_json::json!(3601)),
        ("kek_base64", serde_json::json!("private-invalid-value")),
        ("database_name", serde_json::json!("")),
        ("database_role", serde_json::json!("")),
    ] {
        let mut input = settings();
        input[field] = value;
        save(&path, &input);
        let error = config::load(&path).err().expect("unsafe settings accepted");
        assert!(!error.to_string().contains("private"));
    }
}
