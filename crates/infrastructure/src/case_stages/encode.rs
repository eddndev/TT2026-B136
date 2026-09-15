use super::{inconsistent, port};
use application::case_stages::*;
use application::ApplicationError;
use postgres::Transaction;
use serde_json::{json, Value};

pub(super) fn insert(
    tx: &mut Transaction<'_>,
    snapshot: &CaseStageSnapshot,
) -> Result<(), ApplicationError> {
    let mut value = json!({
        "case_id":snapshot.case_id.to_string(),"revision":snapshot.stage_revision.get(),
        "from_stage":snapshot.from_stage.map(CaseStage::as_str),"stage":snapshot.values.stage().as_str(),
        "administration_revision":snapshot.administration_revision.get(),
        "values_digest":format!("\\x{}",snapshot.values_digest.to_hex()),
        "recorded_at_seconds":snapshot.recorded_at.unix_timestamp(),"recorded_at_nanoseconds":snapshot.recorded_at.nanosecond(),
        "recorded_by":snapshot.recorded_by.id.to_string(),"recorded_by_email":snapshot.recorded_by.email,
    });
    match &snapshot.values {
        CaseStageChange::Adopt(v) => {
            value["change_kind"] = json!("adoption");
            value["reason"] = json!(v.reason().as_str());
            declared(&mut value, "act", v.known_at());
            support(&mut value, "support", find(snapshot, v.support())?);
        }
        CaseStageChange::Transition(StageTransition::ToIntermediate(v)) => {
            value["change_kind"] = json!("to_intermediate");
            value["note"] = json!(v.note().map(StageNote::as_str));
            declared(&mut value, "act", v.accusation_declared_at());
            support(&mut value, "support", find(snapshot, v.accusation())?);
        }
        CaseStageChange::Transition(StageTransition::ToTrial(v)) => {
            value["change_kind"] = json!("to_trial");
            value["note"] = json!(v.note().map(StageNote::as_str));
            value["receiving_court"] = json!(v.receiving_court().as_str());
            value["receipt_reference"] =
                json!(v.receipt_reference().map(StageReceiptReference::as_str));
            declared(&mut value, "act", v.opening_order_issued_at());
            declared(&mut value, "received", v.received_at());
            support(&mut value, "support", find(snapshot, v.opening_order())?);
            if let Some(receipt) = v.receipt_support() {
                support(&mut value, "receipt", find(snapshot, receipt)?);
            }
        }
    }
    // Only fixed server-built fields enter the composite; storage remains typed columns.
    tx.execute("INSERT INTO case_stage_revisions SELECT (jsonb_populate_record(NULL::case_stage_revisions,$1)).*",&[&value]).map_err(port)?;
    Ok(())
}
fn find(
    snapshot: &CaseStageSnapshot,
    reference: StageSupportRef,
) -> Result<&StageSupportSnapshot, ApplicationError> {
    snapshot
        .supports
        .iter()
        .find(|value| {
            value.reference == reference.reference() && value.digest == reference.digest()
        })
        .ok_or_else(|| inconsistent("validated support snapshot is missing"))
}
fn declared(value: &mut Value, prefix: &str, time: DeclaredStageTime) {
    value[format!("{prefix}_offset_seconds")] = json!(time.offset().whole_seconds());
    if let Some(instant) = time.instant_value() {
        value[format!("{prefix}_precision")] = json!("instant");
        value[format!("{prefix}_seconds")] = json!(instant.unix_timestamp());
        value[format!("{prefix}_nanoseconds")] = json!(instant.nanosecond());
    } else {
        value[format!("{prefix}_precision")] = json!("date");
        value[format!("{prefix}_date")] = json!(time.local_date().to_string());
    }
}
fn support(value: &mut Value, prefix: &str, support: &StageSupportSnapshot) {
    for (field, item) in [
        ("id", json!(support.reference.id.to_string())),
        ("version", json!(support.reference.version.get())),
        ("digest", json!(format!("\\x{}", support.digest.to_hex()))),
        ("name", json!(support.name)),
        ("format", json!(support.format.as_str())),
        ("policy", json!(support.policy.as_str())),
    ] {
        value[format!("{prefix}_{field}")] = item;
    }
}
