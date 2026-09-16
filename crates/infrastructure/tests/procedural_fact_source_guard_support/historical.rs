use super::*;
use application::hearing_results::*;
use domain::identity::Role;

#[test]
fn typed_person_sources_bind_exact_subject_digest_and_declared_profile() {
    let Some(mut db) = Fixture::new() else { return };
    let (parent_id, parent_view) = parent(&mut db);
    let mut bundle = typed_participant_database_support::Bundle::new(&db);
    let mut client = postgres::Client::connect(&db.admin_url, postgres::NoTls).unwrap();
    let mut tx = client.transaction().unwrap();
    bundle.seed_document(&mut tx, &db);
    bundle.seed_manual(&mut tx, &db);
    bundle.refresh_stamp(&db);
    bundle.insert(&mut tx, &db);
    tx.commit().unwrap();
    let mut values = notification(parent_id);
    values["intended_recipient"] = json!({"kind":"known","value":{"kind":"participant","id":bundle.participant.to_string(),"revision":2}});
    let mut sources = empty();
    sources["resolution"] = parent_view;
    let row = db.admin.query_one("SELECT encode(t.values_digest,'hex'),encode(s.values_digest,'hex') FROM case_participant_typed_revisions t JOIN case_subject_revisions s ON s.subject_id=t.subject_id AND s.revision=t.subject_revision WHERE t.participant_id=$1 AND t.revision=2", &[&bundle.participant.as_uuid()]).unwrap();
    let digest: String = row.get(0);
    let subject_digest: String = row.get(1);
    sources["participants"] = json!([{"case":db.case.to_string(),"id":bundle.participant.to_string(),"revision":2,
        "values_digest":digest,"status":"active","kind":"defendant","display_name":"Ana",
        "procedural_role":"defendant","organization":null,
        "subject":{"id":bundle.subject.to_string(),"revision":1,"values_digest":subject_digest}}]);
    check(&mut db, "notification", &values, &sources, true);
    for (field, changed) in [
        ("id", json!(Uuid::new_v4())),
        ("revision", json!(2)),
        ("values_digest", json!("00".repeat(32))),
    ] {
        let mut bad = sources.clone();
        bad["participants"][0]["subject"][field] = changed;
        check(&mut db, "notification", &values, &bad, false);
    }
    for (field, changed) in [
        ("kind", json!("victim")),
        ("display_name", json!("Other")),
        ("organization", json!("Other")),
    ] {
        let mut bad = sources.clone();
        bad["participants"][0][field] = changed;
        check(&mut db, "notification", &values, &bad, false);
    }
}
#[test]
fn hearing_result_sources_bind_agreement_content_and_keep_historical_revision() {
    let Some(mut db) = Fixture::new() else { return };
    hearing_database_support::complete(&mut db);
    let scheduling = hearing_database_support::service(&db, db.owner, Role::Owner);
    let hearing = hearing_database_support::persist(
        &scheduling,
        db.case,
        hearing_database_support::schedule(),
    );
    let service = hearing_result_database_support::service(&db, db.owner, Role::Owner);
    let mut command = hearing_result_database_support::record(hearing.snapshot.id);
    let agreement_id = HearingResultAgreementId::new();
    let base_values = hearing_result_database_support::values("Declared session");
    let values = HearingResultValues::new(HearingResultValuesInput {
        occurrence: base_values.occurrence(),
        extent: base_values.extent(),
        event_time: base_values.event_time(),
        summary: base_values.summary().clone(),
        attendees: vec![],
        agreements: vec![HearingResultAgreement::new(
            agreement_id,
            HearingResultText::new("Declared agreement").unwrap(),
        )],
        provenance: base_values.provenance().clone(),
    })
    .unwrap();
    if let HearingResultChange::Record { values: value, .. } = &mut command.change {
        *value = values;
    }
    let result = hearing_result_database_support::persist(&service, db.case, command);
    let snapshot = &result.snapshot;
    let mut value = resolution();
    value["provenance"] = json!({"kind":"hearing_result","reference":{"hearing_id":snapshot.hearing_id.to_string(),
        "result_id":snapshot.id.to_string(),"revision":1,"agreement_id":agreement_id.to_string()},"locator":"Agreement 1","support":null});
    let mut sources = empty();
    sources["hearing_results"] = json!([{"case":db.case.to_string(),"hearing_id":snapshot.hearing_id.to_string(),
        "result_id":snapshot.id.to_string(),"revision":1,"agreement_id":agreement_id.to_string(),
        "values_digest":snapshot.values_digest.to_hex(),"submission_digest":snapshot.receipt.submission_digest.to_hex(),
        "status":"recorded","occurrence":"occurred","event_time":{"precision":"date","date":"2024-12-31","offset_seconds":0},
        "summary":"Declared session","agreement_text":"Declared agreement"}]);
    check(&mut db, "resolution", &value, &sources, true);
    hearing_result_database_support::persist(
        &service,
        db.case,
        HearingResultCommand {
            operation_id: HearingResultOperationId::new(),
            hearing_id: snapshot.hearing_id,
            result_id: snapshot.id,
            change: HearingResultChange::Withdraw {
                expected_revision: HearingResultRevision::initial(),
                reason: HearingResultText::new("Administrative withdrawal").unwrap(),
            },
        },
    );
    check(&mut db, "resolution", &value, &sources, true);
    for (field, changed) in [
        ("agreement_text", json!("Other")),
        ("summary", json!("Other")),
        ("status", json!("withdrawn")),
        ("case", json!(Uuid::new_v4())),
        ("revision", json!(2)),
        ("values_digest", json!("00".repeat(32))),
        ("submission_digest", json!("00".repeat(32))),
        ("agreement_id", json!(Uuid::new_v4())),
    ] {
        let mut bad = sources.clone();
        bad["hearing_results"][0][field] = changed;
        check(&mut db, "resolution", &value, &bad, false);
    }
    let mut bad = value.clone();
    bad["provenance"]["reference"]["agreement_id"] = json!(Uuid::new_v4());
    let mut missing = sources.clone();
    missing["hearing_results"][0]["agreement_id"] =
        bad["provenance"]["reference"]["agreement_id"].clone();
    check(&mut db, "resolution", &bad, &missing, false);
}
