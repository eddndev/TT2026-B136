#![allow(dead_code)]
pub use crate::case_administration_support::Fixture;
use application::cases::*;
use domain::{
    cases::CaseMetadata,
    clock::{Clock, OffsetDateTime},
};
use infrastructure::{PostgresDeadlineStore, RingSha256Hasher};
use std::sync::Arc;
struct FixedClock(OffsetDateTime);
impl Clock for FixedClock {
    fn now(&self) -> OffsetDateTime {
        self.0
    }
}
pub fn store(db: &Fixture) -> PostgresDeadlineStore {
    PostgresDeadlineStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    )
    .unwrap()
}
pub fn snapshot(db: &mut Fixture) -> serde_json::Value {
    db.admin.query_one("SELECT jsonb_build_object(
        'users',(SELECT jsonb_agg(to_jsonb(u) ORDER BY id) FROM users u),
        'memberships',(SELECT jsonb_agg(to_jsonb(m) ORDER BY case_id,user_id) FROM case_memberships m),
        'roots',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM case_deadlines r),
        'revisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY deadline_id,revision) FROM case_deadline_revisions r),
        'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))", &[]).unwrap().get(0)
}
pub fn complete(db: &Fixture) {
    let creation = PenalCaseCreation::new(
        CaseMetadata::new("Responsible case", "REF").unwrap(),
        PenalCaseProfile::new(
            &db.case.to_string(),
            "Authority",
            &db.case.to_string(),
            "Court",
            &["Offense"],
            None,
            None,
        )
        .unwrap(),
    );
    db.store()
        .replace_administration(
            db.owner,
            db.case,
            CaseRevisionExpectation::Unrevised,
            creation.into_values().editable().clone(),
            db.at,
        )
        .unwrap();
}
