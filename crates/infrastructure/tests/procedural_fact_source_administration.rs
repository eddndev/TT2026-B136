mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;
mod hearing_result_database_support;
mod hearing_result_revalidation_support;
mod procedural_fact_backend_support;
use application::{cases::*, procedural_facts::*, ApplicationError};
use domain::{cases::CaseMetadata, identity::Role};
use procedural_fact_backend_support::*;

fn advance(db: &Fixture, revision: u32) -> CaseAdministrationSnapshot {
    let detail = db
        .store()
        .replace_administration(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(revision - 1),
            CaseEditableValues::new(
                CaseMetadata::new(&format!("Administration {revision}"), "REF-OLD").unwrap(),
                None,
            ),
            db.at,
        )
        .unwrap();
    let CurrentCaseAdministration::Recorded(snapshot) = detail.administration else {
        panic!("administration revision expected")
    };
    *snapshot
}

fn corrupt_capture(
    db: &mut Fixture,
    detail: &FactDetail,
    earlier: Option<&CaseAdministrationSnapshot>,
) {
    let (family, id) = match detail.snapshot.target() {
        FactTarget::Resolution(id) => ("resolution", id.as_uuid()),
        FactTarget::Notification { id, .. } => ("notification", id.as_uuid()),
    };
    db.admin
        .batch_execute("ALTER TABLE case_procedural_fact_revisions DISABLE TRIGGER USER")
        .unwrap();
    match earlier {
        None => {
            db.admin.execute("UPDATE case_procedural_fact_revisions SET recorded_administration_revision=NULL,
                recorded_administration_digest=NULL,recorded_administration_title='Baseline',recorded_administration_reference='REF-OLD'
                WHERE family=$1 AND id=$2 AND case_id=$3", &[&family,&id,&db.case.as_uuid()]).unwrap();
        }
        Some(snapshot) => {
            db.admin.execute("UPDATE case_procedural_fact_revisions SET recorded_administration_revision=$4,
                recorded_administration_digest=$5,recorded_administration_title=NULL,recorded_administration_reference=NULL
                WHERE family=$1 AND id=$2 AND case_id=$3", &[&family,&id,&db.case.as_uuid(),&i64::from(snapshot.revision.get()),
                &snapshot.values_digest.as_bytes().as_slice()]).unwrap();
        }
    }
    db.admin
        .batch_execute("ALTER TABLE case_procedural_fact_revisions ENABLE TRIGGER USER")
        .unwrap();
}
fn rejects_without_read_audit(
    db: &mut Fixture,
    workflow: &ProceduralFactService,
    detail: &FactDetail,
) {
    let before = snapshot(db);
    let result = workflow.get(
        "session",
        db.case,
        detail.snapshot.target(),
        Some(detail.snapshot.metadata().revision),
    );
    assert!(
        matches!(
            result,
            Err(ApplicationError::ProceduralFact(
                ProceduralFactError::StoredInconsistent(_)
            ))
        ),
        "{result:?}"
    );
    assert_eq!(snapshot(db), before);
}

#[test]
fn notification_capture_cannot_precede_its_exact_resolution_source() {
    for recorded_earlier in [false, true] {
        let Some(mut db) = Fixture::new() else { return };
        let initial = advance(&db, 1);
        if recorded_earlier {
            advance(&db, 2);
        }
        let workflow = service(&db, db.owner, Role::Owner);
        let resolution = persist(&workflow, db.case, record());
        let notification = persist(&workflow, db.case, notify(resolution_ref(&resolution)));
        assert_eq!(
            notification.snapshot.metadata().recorded_administration,
            resolution.snapshot.metadata().recorded_administration
        );
        corrupt_capture(&mut db, &notification, recorded_earlier.then_some(&initial));
        rejects_without_read_audit(&mut db, &workflow, &notification);
    }
}

#[test]
fn required_revision_case_rejects_a_forged_unrevised_capture() {
    let Some(mut db) = Fixture::new() else { return };
    let anchor = hearing_result_revalidation_support::appointment(&mut db);
    let result = hearing_result_database_support::persist(
        &hearing_result_database_support::service(&db, db.owner, Role::Owner),
        db.case,
        hearing_result_database_support::record(anchor.snapshot.id),
    );
    assert_eq!(result.snapshot.recorded_administration_revision.get(), 1);
    let original = values("Resolution from declared session");
    let values = ResolutionValues::new(ResolutionValuesInput {
        class: original.class().clone(),
        subtype: None,
        issuer: original.issuer().clone(),
        issued_at: original.issued_at(),
        summary: original.summary().clone(),
        provenance: FactProvenance::HearingResult {
            reference: FactHearingRef {
                hearing_id: result.snapshot.hearing_id,
                result_id: result.snapshot.id,
                revision: result.snapshot.revision,
                agreement_id: None,
            },
            locator: FactLabel::new("Declared session").unwrap(),
            support: None,
        },
    });
    let workflow = service(&db, db.owner, Role::Owner);
    let resolution = persist(
        &workflow,
        db.case,
        ProceduralFactCommand::Resolution(ResolutionCommand::new(
            FactOperationId::new(),
            ResolutionId::new(),
            FactChange::record(values),
        )),
    );
    corrupt_capture(&mut db, &resolution, None);
    rejects_without_read_audit(&mut db, &workflow, &resolution);
}

#[test]
fn resolution_capture_cannot_precede_its_exact_result_source() {
    let Some(mut db) = Fixture::new() else { return };
    let anchor = hearing_result_revalidation_support::appointment(&mut db);
    let initial = db
        .store()
        .get_administration(db.owner, db.case, db.at)
        .unwrap();
    let CurrentCaseAdministration::Recorded(initial) = initial.administration else {
        panic!("initial administration revision expected")
    };
    assert_eq!(initial.revision.get(), 1);
    let advanced = db
        .store()
        .replace_administration(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(1),
            initial.values.editable().clone(),
            db.at,
        )
        .unwrap();
    let CurrentCaseAdministration::Recorded(advanced) = advanced.administration else {
        panic!("second administration revision expected")
    };
    assert_eq!(advanced.revision.get(), 2);
    let result = hearing_result_database_support::persist(
        &hearing_result_database_support::service(&db, db.owner, Role::Owner),
        db.case,
        hearing_result_database_support::record(anchor.snapshot.id),
    );
    assert_eq!(result.snapshot.recorded_administration_revision.get(), 2);
    assert_eq!(
        result.snapshot.recorded_administration_digest,
        advanced.values_digest
    );
    let original = values("Resolution from declared session");
    let values = ResolutionValues::new(ResolutionValuesInput {
        class: original.class().clone(),
        subtype: None,
        issuer: original.issuer().clone(),
        issued_at: original.issued_at(),
        summary: original.summary().clone(),
        provenance: FactProvenance::HearingResult {
            reference: FactHearingRef {
                hearing_id: result.snapshot.hearing_id,
                result_id: result.snapshot.id,
                revision: result.snapshot.revision,
                agreement_id: None,
            },
            locator: FactLabel::new("Declared session").unwrap(),
            support: None,
        },
    });
    let workflow = service(&db, db.owner, Role::Owner);
    let resolution = persist(
        &workflow,
        db.case,
        ProceduralFactCommand::Resolution(ResolutionCommand::new(
            FactOperationId::new(),
            ResolutionId::new(),
            FactChange::record(values),
        )),
    );
    assert_eq!(
        resolution.snapshot.metadata().recorded_administration,
        CurrentCaseAdministration::Recorded(advanced)
    );
    workflow
        .get(
            "session",
            db.case,
            resolution.snapshot.target(),
            Some(resolution.snapshot.metadata().revision),
        )
        .unwrap();
    corrupt_capture(&mut db, &resolution, Some(&initial));
    rejects_without_read_audit(&mut db, &workflow, &resolution);
}
