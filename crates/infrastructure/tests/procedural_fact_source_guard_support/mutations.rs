use super::{procedural_fact_backend_support as backend, *};
use application::procedural_facts::*;
use domain::{crypto::DocumentHasher, identity::Role};
use infrastructure::RingSha256Hasher;

fn rejected_correction(db: &mut Fixture, base: &FactDetail, selected: FactSources) {
    let ProceduralFactSnapshot::Notification(snapshot) = &base.snapshot else {
        panic!("notification required")
    };
    let command = ProceduralFactCommand::Notification(
        NotificationCommand::new(
            FactOperationId::new(),
            snapshot.root.id(),
            snapshot.root.resolution_id(),
            FactChange::correct(
                snapshot.metadata.revision,
                snapshot.values.clone(),
                backend::text("Correct capture"),
            ),
        )
        .unwrap(),
    );
    let values = snapshot.values.canonical_bytes();
    let sources = fact_sources_bytes(&selected).unwrap();
    let receipt = fact_submission_bytes(
        db.owner,
        db.case,
        &command,
        RingSha256Hasher.hash_bytes(&values),
        RingSha256Hasher.hash_bytes(&sources),
    )
    .unwrap();
    let before = backend::counts(db);
    let error = db.admin.execute("INSERT INTO case_procedural_fact_revisions(family,id,case_id,revision,values_canonical,values_digest,sources_canonical,sources_digest,operation_id,action,reason,submission_canonical,submission_digest,recorded_administration_title,recorded_administration_reference,recorded_at_seconds,recorded_at_nanoseconds,recorded_by,recorded_by_email) VALUES('notification',$1,$2,2,$3,sha256($3),$4,sha256($4),$5,'correct','Correct capture',$6,sha256($6),'Baseline','REF-OLD',1735689600,0,$7,'owner@example.test')",&[&snapshot.root.id().as_uuid(),&db.case.as_uuid(),&values,&sources,&command.operation_id().as_uuid(),&receipt,&db.owner.as_uuid()]).unwrap_err();
    assert_eq!(
        error.code().map(|code| code.code()),
        Some("23514"),
        "{error}"
    );
    assert_eq!(backend::counts(db), before);
}
#[test]
fn direct_sql_cannot_omit_substitute_or_rewrite_a_parent_even_with_recomputed_receipt() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = backend::service(&db, db.owner, Role::Owner);
    let first = backend::persist(&workflow, db.case, backend::record());
    let other = backend::persist(&workflow, db.case, backend::record());
    let notice = backend::persist(
        &workflow,
        db.case,
        backend::notify(backend::resolution_ref(&first)),
    );
    let other_notice = backend::persist(
        &workflow,
        db.case,
        backend::notify(backend::resolution_ref(&other)),
    );
    let mut missing = notice.sources.clone();
    missing.resolved.resolution = None;
    missing.views.resolution = None;
    rejected_correction(&mut db, &notice, missing);
    let mut replacement = notice.sources.clone();
    replacement.resolved.resolution = other_notice.sources.resolved.resolution;
    replacement.views.resolution = other_notice.sources.views.resolution.clone();
    rejected_correction(&mut db, &notice, replacement);
    let mut rewritten = notice.sources.clone();
    rewritten.views.resolution.as_mut().unwrap().summary =
        backend::text("Rewritten historical summary");
    rejected_correction(&mut db, &notice, rewritten);
}
