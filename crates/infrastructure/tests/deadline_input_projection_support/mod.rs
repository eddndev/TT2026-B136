#![allow(dead_code)]
use crate::case_administration_support::Fixture;
use serde_json::{json, Value};
use uuid::Uuid;

pub fn text(value: &str) -> Vec<u8> {
    let mut bytes = (value.len() as u32).to_be_bytes().to_vec();
    bytes.extend_from_slice(value.as_bytes());
    bytes
}
pub fn unknown(value: &str) -> Vec<u8> {
    let mut bytes = vec![0];
    bytes.extend(text(value));
    bytes
}
pub fn reference(id: Uuid, revision: u32) -> Vec<u8> {
    let mut bytes = id.as_bytes().to_vec();
    bytes.extend(revision.to_be_bytes());
    bytes
}
pub fn known(family: u8, reference: &[u8]) -> Vec<u8> {
    let mut bytes = vec![1, family];
    bytes.extend_from_slice(reference);
    bytes
}
pub fn qualification(time: &[u8]) -> Vec<u8> {
    let mut bytes = vec![1, 1];
    bytes.extend_from_slice(time);
    bytes.extend(text("Declared start"));
    bytes.extend(text("Page 1"));
    bytes
}
#[derive(Clone)]
pub struct Input {
    pub case: Uuid,
    pub source: Vec<u8>,
    pub qualification: Vec<u8>,
    pub calendar: Vec<u8>,
    pub ordered: Vec<u8>,
    pub statement: String,
    pub locator: String,
    pub scope: Vec<u8>,
    pub incident: Vec<u8>,
    pub conditions: Vec<(Uuid, Vec<u8>, String)>,
}
impl Default for Input {
    fn default() -> Self {
        Self {
            case: Uuid::nil(),
            source: unknown("x"),
            qualification: vec![0],
            calendar: vec![0],
            ordered: vec![0],
            statement: "x".into(),
            locator: "x".into(),
            scope: vec![1, 1],
            incident: vec![1, 0],
            conditions: vec![],
        }
    }
}
impl Input {
    pub fn bytes(&self) -> Vec<u8> {
        let mut bytes = b"DEVI1".to_vec();
        bytes.extend(self.case.as_bytes());
        for field in [
            &self.source,
            &self.qualification,
            &self.calendar,
            &self.ordered,
        ] {
            bytes.extend(field);
        }
        bytes.extend(text(&self.statement));
        bytes.extend(text(&self.locator));
        bytes.extend(&self.scope);
        bytes.extend(&self.incident);
        bytes.extend((self.conditions.len() as u32).to_be_bytes());
        for (id, applies, locator) in &self.conditions {
            bytes.extend(id.as_bytes());
            bytes.extend(applies);
            bytes.extend(text(locator));
        }
        bytes
    }
}
pub fn expected(case: Uuid) -> Value {
    json!({"case_id": case, "source_kind": null, "source_id": null, "source_revision": null,
        "source_hearing_id": null, "source_parent_resolution_id": null, "source_parent_resolution_revision": null,
        "source_agreement_id": null, "calendar_id": null, "calendar_revision": null})
}
pub fn project(db: &mut Fixture, bytes: &[u8]) -> Result<Value, postgres::Error> {
    db.admin
        .query_one("SELECT deadline_input_selection($1)", &[&bytes])
        .map(|row| row.get(0))
}
pub fn valid(db: &mut Fixture, input: &Input, expected: Value) {
    let bytes = input.bytes();
    application::deadline_evaluations::decode_deadline_evaluation_input(&bytes).unwrap();
    assert_eq!(project(db, &bytes).unwrap(), expected);
}
pub fn rejected(db: &mut Fixture, bytes: &[u8]) {
    assert!(
        project(db, bytes).is_err(),
        "SQL accepted malformed DEVI1 of {} bytes",
        bytes.len()
    );
}
