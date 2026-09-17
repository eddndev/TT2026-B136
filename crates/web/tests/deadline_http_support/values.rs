use domain::{cases::CaseId, crypto::Sha256Digest};
use serde_json::{json, Value};
pub const CASE: &str = "00000000-0000-0000-0000-000000000001";
pub const ID: &str = "00000000-0000-0000-0000-000000000000";
pub const PROFILE: &str = "00000000-0000-0000-0000-000000000002";
pub const SOURCE: &str = "00000000-0000-0000-0000-000000000003";
pub const ACTOR: &str = "00000000-0000-0000-0000-000000000004";
pub const BASE: &str = "/api/v1/cases/00000000-0000-0000-0000-000000000001/deadlines";
pub fn case() -> CaseId {
    CaseId::from_uuid(CASE.parse().unwrap())
}
pub fn digest() -> Sha256Digest {
    Sha256Digest::from_array([7; 32])
}
pub fn input() -> Value {
    json!({"selection":{"case_id":CASE,"source":{"kind":"known","value":{"family":"resolution","id":SOURCE,"revision":1}},"qualification":null},
        "calendar":null,"ordered_quantity":null,"qualification":{"statement":"Declared applicability","locator":"Resolution page 1",
        "scope_applies":{"kind":"known","value":true},"unresolved_incident":{"kind":"known","value":false},
        "conditions":[{"id":ID,"applies":{"kind":"known","value":true},"locator":"Condition record"}]}})
}
pub fn definition() -> Value {
    json!({"title":"Declared period","profile":{"id":PROFILE,"revision":1},"responsible_id":ACTOR,"input":input()})
}
pub fn command() -> Value {
    json!({"operation_id":ID,"deadline_id":ID,"change":{"action":"register","expected_revision":0,"definition":definition()}})
}
pub fn submission() -> Value {
    json!({"command":command(),"expected_submission_digest":digest().to_hex()})
}
