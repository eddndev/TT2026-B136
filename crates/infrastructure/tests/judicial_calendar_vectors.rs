mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod judicial_calendar_database_support;
use application::judicial_calendars::*;
use domain::identity::Role;
use judicial_calendar_database_support::*;
use serde_json::Value;

#[test]
fn all_independent_value_vectors_roundtrip_through_generated_sql_views_and_rust_decoder() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let fixtures: Vec<Value> = serde_json::from_str(include_str!(
        "../../domain/tests/fixtures/judicial_calendar_vectors.json"
    ))
    .unwrap();
    for fixture in fixtures {
        let encoded = fixture["hex"].as_str().unwrap();
        let canonical = (0..encoded.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&encoded[i..i + 2], 16).unwrap())
            .collect::<Vec<_>>();
        let values = JudicialCalendarValues::from_canonical_bytes(&canonical).unwrap();
        let record = persist(
            &workflow,
            JudicialCalendarCommand {
                calendar_id: JudicialCalendarId::new(),
                operation_id: JudicialCalendarOperationId::new(),
                change: JudicialCalendarChange::Publish { values },
            },
        );
        assert_eq!(
            record.values_digest.to_hex(),
            fixture["sha256"].as_str().unwrap()
        );
        let row=db.admin.query_one("SELECT values_view,values_canonical FROM judicial_calendar_revisions WHERE calendar_id=$1",&[&record.id.as_uuid()]).unwrap();
        assert_eq!(
            row.get::<_, Value>(0),
            fixture["normalized"],
            "{}",
            fixture["name"]
        );
        assert_eq!(row.get::<_, Vec<u8>>(1), canonical);
        assert_eq!(
            workflow
                .get("session", record.id, Some(record.revision))
                .unwrap(),
            record
        );
    }
    assert_eq!(counts(&mut db).0, 7);
}
