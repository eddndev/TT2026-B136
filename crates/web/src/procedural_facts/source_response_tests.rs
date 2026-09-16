use super::{internal, project};
#[path = "source_test_support.rs"]
mod fixture;
use domain::hearing_results::DeclaredHearingResultTime;
use fixture::rich_detail;
use serde_json::json;

#[test]
fn exact_archived_identity_hearing_agreement_and_admission_remain_readable() {
    let row = rich_detail();
    let participant = row.sources.resolved.participants[0];
    let hearing = row.sources.resolved.hearing_results[0];
    let document = &row.sources.direct_supports[0];
    let value = project(row.clone()).unwrap();
    let sources = &value["sources"];
    assert_eq!(
        sources["participants"][0],
        json!({"case_id":participant.case_id,
        "id":participant.reference.id.to_string(),"revision":7,"values_digest":participant.values_digest.to_hex(),
        "directory_status":"archived","subject":{"id":participant.subject.unwrap().id.to_string(),"revision":3,"values_digest":participant.subject.unwrap().values_digest.to_hex()},
        "display_name":"Historical counsel","procedural_role":"defense_counsel","organization":"Office","kind":"defense_counsel"})
    );
    assert_eq!(
        sources["hearing_results"][0]["agreement_id"],
        uuid::Uuid::nil().to_string()
    );
    assert_eq!(
        sources["hearing_results"][0]["agreement"],
        json!({"id":uuid::Uuid::nil().to_string(),"text":"Exact selected agreement"})
    );
    assert_eq!(sources["hearing_results"][0]["status"], "withdrawn");
    assert_eq!(
        sources["hearing_results"][0]["submission_digest"],
        hearing.submission_digest.to_hex()
    );
    assert_eq!(
        sources["hearing_results"][0]["event_time"],
        json!({"precision":"date","year":2026,"month":9,"day":16,"offset_seconds":-21600})
    );
    assert_eq!(
        sources["direct_supports"][0],
        json!({"document_id":document.reference.id.to_string(),"version":3,"digest":document.digest.to_hex(),"name":"notice.pdf","format":"pdf","policy":"pdf_docx_v1"})
    );
    assert_eq!(
        value["values"]["provenance"]["support"]["locator"],
        "Page 1"
    );
}
#[test]
fn contradictory_participant_and_result_views_never_reach_json() {
    let mut row = rich_detail();
    row.sources.views.participants[0].subject = None;
    internal(project(row));
    let mut row = rich_detail();
    row.sources.views.hearing_results[0].agreement = None;
    internal(project(row));
    let mut row = rich_detail();
    row.sources.views.participants.clear();
    internal(project(row));
    let mut row = rich_detail();
    row.sources.direct_supports[0].name = "../notice.pdf".into();
    internal(project(row));
}
#[test]
fn hearing_instant_keeps_local_components_and_original_offset() {
    let mut row = rich_detail();
    let instant = time::Date::from_calendar_date(2026, time::Month::September, 16)
        .unwrap()
        .with_hms(1, 2, 3)
        .unwrap()
        .assume_offset(time::UtcOffset::from_hms(14, 0, 0).unwrap());
    row.sources.views.hearing_results[0].event_time =
        DeclaredHearingResultTime::instant(instant).unwrap();
    let value = project(row).unwrap();
    assert_eq!(
        value["sources"]["hearing_results"][0]["event_time"],
        json!({"precision":"instant","year":2026,"month":9,"day":16,"hour":1,"minute":2,"second":3,"offset_seconds":50400})
    );
}
