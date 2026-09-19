#![allow(dead_code)]
#[path = "../case_administration_support/mod.rs"]
mod administration;
pub use administration::Fixture;
use domain::identity::UserId;
use infrastructure::{PostgresMemberStore, SystemClock};
use std::sync::Arc;

pub fn store(db: &Fixture) -> PostgresMemberStore {
    PostgresMemberStore::open(&db.runtime_url, Arc::new(SystemClock)).unwrap()
}

pub fn user(db: &mut Fixture, email: &str, role: &str, active: bool, assigned: bool) -> UserId {
    let id = UserId::new();
    db.admin.execute("INSERT INTO users(id,email,password_hash,role,active,protected_totp_secret,recovery_codes)
        VALUES($1,$2,'fixture',$3,$4,'\\x00','{}')", &[&id.as_uuid(), &email, &role, &active]).unwrap();
    if assigned {
        db.admin
            .execute(
                "INSERT INTO case_memberships(case_id,user_id) VALUES($1,$2)",
                &[&db.case.as_uuid(), &id.as_uuid()],
            )
            .unwrap();
    }
    id
}

pub fn snapshot(db: &mut Fixture) -> serde_json::Value {
    db.admin
        .query_one(
            "SELECT jsonb_build_object(
        'users',(SELECT jsonb_agg(to_jsonb(u) ORDER BY id) FROM users u),
        'members',(SELECT jsonb_agg(to_jsonb(m) ORDER BY case_id,user_id) FROM case_memberships m),
        'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))",
            &[],
        )
        .unwrap()
        .get(0)
}
