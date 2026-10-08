#![allow(dead_code, unused_imports)]
pub use crate::record_decision_support::*;
pub use application::ApplicationError;
pub use domain::{cases::CaseId, crypto::Sha256Digest};
pub use serde_json::{json, Value};
pub use time::Duration;
pub use uuid::Uuid;

#[path = "../../../application/tests/measure_record_read_support/fixtures.rs"]
mod fixtures;
pub use fixtures::*;
mod ports;
pub use ports::*;
mod bindings;
mod lifecycle;
mod pagination;
mod transport;

pub fn case_id() -> CaseId {
    crate::context_support::initial().case_id
}
pub fn base() -> String {
    format!("/api/v1/cases/{}/measures", case_id())
}
pub fn current_path(id: MeasureId) -> String {
    format!("{}/{}", base(), id)
}
pub fn exact_path(reference: PrecautionaryMeasureRef) -> String {
    format!(
        "{}/revisions/{}?capture_digest={}",
        current_path(reference.id()),
        reference.revision().get(),
        reference.digest().to_hex()
    )
}
pub fn reference_json(reference: PrecautionaryMeasureRef) -> Value {
    json!({"id":reference.id().to_string(),"revision":reference.revision().get(),
        "capture_digest":reference.digest().to_hex()})
}
pub fn internal() -> Value {
    json!({"error":{"code":"internal_error","message":"internal application error"}})
}
pub fn current_port(row: MeasureRecordDetail) -> MockRecords {
    let mut port = MockRecords::new();
    port.expect_get()
        .times(1)
        .return_once(move |token, case, id| {
            assert_eq!(
                (token, case, id),
                ("staff-token", row.case_id, row.reference.id())
            );
            Ok(row)
        });
    port
}
pub fn exact_port(row: MeasureRecordDetail) -> MockRecords {
    let mut port = MockRecords::new();
    port.expect_exact()
        .times(1)
        .return_once(move |token, case, reference| {
            assert_eq!(
                (token, case, reference),
                ("staff-token", row.case_id, row.reference)
            );
            Ok(row)
        });
    port
}
pub fn page(items: Vec<MeasureRecordDetail>) -> MeasureRecordPage {
    MeasureRecordPage {
        case_id: case_id(),
        items,
        has_more: false,
        next_after_id: None,
    }
}
