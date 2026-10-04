use crate::{deadline_http_support as deadlines, hearing_result_support as results};
pub use results::{case, CASE, HEARING, RESULT};
use serde_json::{json, Value};
pub fn base() -> String {
    format!("{}/derived-deadline", results::base_url())
}
pub fn command() -> Value {
    let mut deadline = deadlines::command();
    let input = &mut deadline["change"]["definition"]["input"];
    input["selection"]["case_id"] = json!(CASE);
    input["selection"]["source"] = json!({"kind":"known","value":{
        "family":"hearing_result","hearing_id":HEARING,"result_id":RESULT,
        "revision":1,"agreement_id":null}});
    json!({"case_id":CASE,"result":results::command_json(),"deadline":deadline})
}
pub fn submission() -> Value {
    json!({"command":command(),"expected_review_digest":deadlines::digest().to_hex()})
}
