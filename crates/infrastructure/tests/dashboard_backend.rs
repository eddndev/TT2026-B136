#[path = "case_administration_support/mod.rs"]
mod case_administration_support;
#[path = "case_stage_database_support/mod.rs"]
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
#[path = "deadline_backend_support/mod.rs"]
mod deadline_backend_support;
#[path = "deadline_profile_database_support/mod.rs"]
mod deadline_profile_database_support;
#[allow(dead_code)]
#[path = "document_store_support/mod.rs"]
mod document_store_support;
#[path = "hearing_database_support/mod.rs"]
mod hearing_database_support;
#[path = "procedural_fact_backend_support/mod.rs"]
mod procedural_fact_backend_support;

use application::{cases::*, dashboard::*, deadlines::*, documents::*, ApplicationError};
use case_administration_support::Fixture;
use case_stage_database_support::FixedClock;
use deadline_backend_support as dl;
use domain::{crypto::DocumentVersion, identity::Role};
use infrastructure::{PostgresCaseDocumentStore, PostgresDashboardStore, RingSha256Hasher};
use std::sync::Arc;
use time::{Duration, OffsetDateTime};

fn store(db: &Fixture, at: OffsetDateTime) -> PostgresDashboardStore {
    PostgresDashboardStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(at)),
    )
    .unwrap()
}
fn audits(db: &mut Fixture) -> i64 {
    db.admin
        .query_one(
            "SELECT count(*) FROM audit_events WHERE action='dashboard.read'",
            &[],
        )
        .unwrap()
        .get(0)
}
fn fixture() -> Option<Fixture> {
    let mut db = Fixture::new()?;
    hearing_database_support::complete(&mut db);
    Some(db)
}
fn accepted(db: &Fixture) -> (application::procedural_facts::FactDetail, DeadlineDetail) {
    let profile = dl::profile(db);
    let source = dl::source(db);
    let command = dl::command(db, &profile, &source);
    let value = dl::persist(
        &dl::service(db, db.owner, Role::Owner),
        db.case,
        dl::human(command, Some(dl::FOLLOW_RESOLUTION)),
    );
    (source, value)
}
#[test]
fn office_and_assigned_scope_never_leak_unrelated_members_or_cases() {
    let Some(mut db) = fixture() else { return };
    let ours = db.case;
    let litigator = db.user("litigator", true);
    let colleague = db.user("litigator", true);
    let unassigned = db.user("litigator", false);
    let paralegal = db.user("paralegal", true);
    let client = db.user("client", true);
    hearing_database_support::complete(&mut db);
    let unrelated = db.user("litigator", true);
    let store = store(&db, db.at);
    let office = store.read(db.owner).unwrap();
    assert_eq!(office.scope, DashboardScope::Office);
    assert_eq!(office.active_cases, 3);
    assert_eq!(office.workload.len(), 4);
    assert_eq!(
        office
            .workload
            .iter()
            .find(|v| v.user_id == unassigned)
            .unwrap()
            .active_cases,
        0
    );
    assert!(office
        .workload
        .windows(2)
        .all(|p| p[0].user_id.as_uuid() < p[1].user_id.as_uuid()));
    let assigned = store.read(litigator).unwrap();
    assert_eq!(assigned.scope, DashboardScope::AssignedCases);
    assert_eq!(assigned.active_cases, 1);
    assert_eq!(assigned.workload.len(), 2);
    assert!(assigned
        .workload
        .iter()
        .all(|v| [litigator, colleague].contains(&v.user_id) && v.active_cases == 1));
    assert!(!assigned.workload.iter().any(|v| v.user_id == unrelated));
    let empty = store.read(unassigned).unwrap();
    assert_eq!(empty.active_cases, 0);
    assert_eq!(empty.pending_contracts, 0);
    assert!(empty.workload.is_empty());
    for denied in [paralegal, client] {
        assert!(matches!(
            store.read(denied),
            Err(ApplicationError::PermissionDenied)
        ));
    }
    db.store()
        .remove_member(ours, litigator, db.owner, db.at)
        .unwrap();
    assert_eq!(store.read(litigator).unwrap().active_cases, 0);
    db.admin.execute("UPDATE users SET revision=revision+1,auth_generation=auth_generation+1,active=false WHERE id=$1", &[&colleague.as_uuid()]).unwrap();
    assert!(matches!(
        store.read(colleague),
        Err(ApplicationError::InvalidSession)
    ));
    assert!(!store
        .read(db.owner)
        .unwrap()
        .workload
        .iter()
        .any(|v| v.user_id == colleague));
}
#[test]
fn contract_totals_use_latest_version_and_classification_without_name_inference() {
    let Some(mut db) = fixture() else { return };
    let litigator = db.user("litigator", true);
    let docs = PostgresCaseDocumentStore::open(&db.runtime_url).unwrap();
    let mut contract = document_store_support::document();
    contract.name = "unrelated-name.txt".into();
    docs.insert_with_metadata(
        db.owner,
        db.case,
        contract.clone(),
        DocumentMetadata::new(Some("  CoNtRaTo  "), None, &[]).unwrap(),
        db.at,
    )
    .unwrap();
    let mut decoy = document_store_support::document();
    decoy.name = "contract.txt".into();
    docs.insert(db.owner, db.case, decoy, db.at).unwrap();
    let dashboard = store(&db, db.at);
    assert_eq!(dashboard.read(litigator).unwrap().pending_contracts, 1);
    let mut sealed = contract.clone();
    sealed.seal(document_store_support::evidence()).unwrap();
    docs.seal(db.owner, db.case, sealed, db.at).unwrap();
    assert_eq!(dashboard.read(litigator).unwrap().pending_contracts, 0);
    let mut next = contract.clone();
    next.version = DocumentVersion::new(2).unwrap();
    docs.append(db.owner, db.case, contract.version, next, db.at)
        .unwrap();
    assert_eq!(dashboard.read(litigator).unwrap().pending_contracts, 1);
    docs.replace_metadata(
        db.owner,
        db.case,
        contract.id,
        MetadataRevision::new(1),
        DocumentMetadata::new(Some("Other"), None, &[]).unwrap(),
        db.at,
    )
    .unwrap();
    assert_eq!(dashboard.read(litigator).unwrap().pending_contracts, 0);
    docs.replace_metadata(
        db.owner,
        db.case,
        contract.id,
        MetadataRevision::new(2),
        DocumentMetadata::new(Some("CONTRACT"), None, &[]).unwrap(),
        db.at,
    )
    .unwrap();
    assert_eq!(dashboard.read(litigator).unwrap().pending_contracts, 1);
    hearing_database_support::complete(&mut db);
    docs.insert_with_metadata(
        db.owner,
        db.case,
        document_store_support::document(),
        DocumentMetadata::new(Some("contract"), None, &[]).unwrap(),
        db.at,
    )
    .unwrap();
    assert_eq!(dashboard.read(litigator).unwrap().pending_contracts, 1);
    assert_eq!(dashboard.read(db.owner).unwrap().pending_contracts, 2);
}
#[test]
fn case_capacity_failure_is_explicit_and_does_not_return_a_partial_count() {
    let Some(mut db) = Fixture::new() else { return };
    db.admin.execute("INSERT INTO cases(id,title,reference,created_by,required_initial_revision) SELECT md5(n::text)::uuid,'Capacity','CAPACITY-'||n,$1,NULL FROM generate_series(1,1001) n", &[&db.owner.as_uuid()]).unwrap();
    assert!(
        matches!(store(&db, db.at).read(db.owner), Err(ApplicationError::Port(message)) if message.contains("capacity"))
    );
    assert_eq!(audits(&mut db), 0);
}

#[path = "dashboard_concurrency.rs"]
mod dashboard_concurrency;
#[path = "dashboard_deadlines.rs"]
mod dashboard_deadlines;
