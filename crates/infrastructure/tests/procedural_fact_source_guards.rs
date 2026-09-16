mod case_administration_support;
mod procedural_fact_source_guard_support;
#[allow(dead_code)]
mod procedural_fact_sql_support;
use case_administration_support::Fixture;
use procedural_fact_source_guard_support::*;
use serde_json::json;
use uuid::Uuid;

#[test]
fn source_free_resolution_accepts_only_its_empty_immediate_selection() {
    let Some(mut db) = Fixture::new() else { return };
    check(&mut db, "resolution", &resolution(), &empty(), true);
    let (_, item) = document(&mut db);
    let mut sources = empty();
    sources["direct_supports"] = json!([item]);
    check(&mut db, "resolution", &resolution(), &sources, false);
    check(&mut db, "unknown", &resolution(), &empty(), false);
}
#[test]
fn notification_requires_its_exact_parent_and_matching_readable_snapshot() {
    let Some(mut db) = Fixture::new() else { return };
    let (id, projection) = parent(&mut db);
    let value = notification(id);
    let mut sources = empty();
    sources["resolution"] = projection.clone();
    check(&mut db, "notification", &value, &sources, true);
    check(&mut db, "notification", &value, &empty(), false);
    check(&mut db, "resolution", &resolution(), &sources, false);
    for (field, changed) in [
        ("id", json!(Uuid::new_v4())),
        ("case", json!(Uuid::new_v4())),
        ("revision", json!(2)),
        ("status", json!("withdrawn")),
        ("values_digest", json!("aa".repeat(32))),
        ("submission_digest", json!("bb".repeat(32))),
        ("summary", json!("Different")),
        ("issuer", json!({"known":"Other court"})),
        (
            "issued_at",
            json!({"precision":"date","date":"2025-01-01","offset_seconds":null}),
        ),
    ] {
        let mut changed_sources = sources.clone();
        changed_sources["resolution"][field] = changed;
        check(&mut db, "notification", &value, &changed_sources, false);
    }
}
#[test]
fn manual_people_are_exact_deduplicated_revisions_not_current_directory_heads() {
    let Some(mut db) = Fixture::new() else { return };
    let (parent_id, parent_view) = parent(&mut db);
    let (id, participant) = manual(&mut db);
    let mut value = notification(parent_id);
    value["intended_recipient"] =
        json!({"kind":"known","value":{"kind":"participant","id":id,"revision":1}});
    value["actual_receiver"] = value["intended_recipient"].clone();
    let mut sources = empty();
    sources["resolution"] = parent_view;
    sources["participants"] = json!([participant]);
    check(&mut db, "notification", &value, &sources, true);
    db.admin.execute("INSERT INTO case_participant_revisions(participant_id,revision,display_name,procedural_role,directory_status,values_digest,changed_at,changed_by,changed_by_email) VALUES($1,2,'Edited person','Declared role','archived',sha256(participant_values_bytes('Edited person','Declared role',NULL,NULL,'archived')),'2025-01-02T00:00:00Z',$2,'owner@example.test')", &[&id,&db.owner.as_uuid()]).unwrap();
    check(&mut db, "notification", &value, &sources, true);
    for (field, changed) in [
        ("display_name", json!("Edited person")),
        ("status", json!("archived")),
        ("case", json!(Uuid::new_v4())),
        ("values_digest", json!("00".repeat(32))),
        ("procedural_role", json!("Other role")),
        ("organization", json!("Other")),
    ] {
        let mut changed_sources = sources.clone();
        changed_sources["participants"][0][field] = changed;
        check(&mut db, "notification", &value, &changed_sources, false);
    }
    let mut extra = sources.clone();
    extra["participants"]
        .as_array_mut()
        .unwrap()
        .push(sources["participants"][0].clone());
    check(&mut db, "notification", &value, &extra, false);
    sources["participants"] = json!([]);
    check(&mut db, "notification", &value, &sources, false);
}
#[test]
fn documentary_sources_bind_selected_case_version_digest_and_name() {
    let Some(mut db) = Fixture::new() else { return };
    let (evidence, support) = document(&mut db);
    let mut value = resolution();
    value["provenance"] =
        json!({"kind":"external_reference","reference":"Declared source","support":evidence});
    let mut sources = empty();
    sources["direct_supports"] = json!([support]);
    check(&mut db, "resolution", &value, &sources, true);
    for (field, changed) in [
        ("id", json!(Uuid::new_v4())),
        ("version", json!(2)),
        ("digest", json!("00".repeat(32))),
        ("name", json!("other.pdf")),
    ] {
        let mut changed_sources = sources.clone();
        changed_sources["direct_supports"][0][field] = changed;
        check(&mut db, "resolution", &value, &changed_sources, false);
    }
    check(&mut db, "resolution", &value, &empty(), false);
}

mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;
mod hearing_result_database_support;
#[path = "procedural_fact_source_guard_support/historical.rs"]
mod historical;
mod typed_participant_database_support;

#[path = "procedural_fact_source_guard_support/mutations.rs"]
mod mutations;
mod procedural_fact_backend_support;

#[path = "procedural_fact_source_guard_support/two_documents.rs"]
mod two_documents;
