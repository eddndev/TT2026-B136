#![allow(dead_code)]

use crate::deadline_input_support::*;
use application::{cases::*, deadline_inputs::*, procedural_facts::*, ApplicationError};
use domain::{cases::CaseMetadata, identity::Role};
use infrastructure::{deadline_input_history::load_captured_material, RingSha256Hasher};

pub fn historical(
    db: &Fixture,
    request: &DeadlineInputRequest,
    captured: &DeadlineInputMaterial,
    heads: &DeadlineInputHeads,
) -> Result<DeadlineInputMaterial, ApplicationError> {
    let mut client = db.runtime();
    let mut tx = client.transaction().unwrap();
    let result = load_captured_material(
        &mut tx,
        request.requirement,
        &request.trigger,
        request.calendar,
        &captured.administration,
        heads,
        &RingSha256Hasher,
    );
    tx.rollback().unwrap();
    result
}

pub fn advance_administration(db: &Fixture, expected: u32) -> CurrentCaseAdministration {
    db.store()
        .replace_administration(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(expected),
            CaseEditableValues::new(CaseMetadata::new("Changed case", "NEW-REF").unwrap(), None),
            db.at,
        )
        .unwrap()
        .administration
}

pub fn notice(db: &Fixture, parent: FactResolutionRef, base: Option<&FactDetail>) -> FactDetail {
    let values = notification_at(parent, "2026-01-09");
    let (id, change) = if let Some(base) = base {
        let ProceduralFactSnapshot::Notification(snapshot) = &base.snapshot else {
            panic!("notification expected")
        };
        (
            snapshot.root.id(),
            FactChange::correct(
                snapshot.metadata.revision,
                values,
                FactText::new("Correct exact parent").unwrap(),
            ),
        )
    } else {
        (NotificationId::new(), FactChange::record(values))
    };
    crate::procedural_fact_backend_support::persist(
        &crate::procedural_fact_backend_support::service(db, db.owner, Role::Owner),
        db.case,
        ProceduralFactCommand::Notification(
            NotificationCommand::new(FactOperationId::new(), id, parent.id, change).unwrap(),
        ),
    )
}

pub fn corrupt_receipt(db: &mut Fixture, detail: &FactDetail) {
    let FactTarget::Resolution(id) = detail.snapshot.target() else {
        panic!("resolution expected")
    };
    db.admin.batch_execute(
        "ALTER TABLE case_procedural_fact_revisions DISABLE TRIGGER USER;
         ALTER TABLE case_procedural_fact_revisions DROP CONSTRAINT procedural_fact_submission_hash",
    ).unwrap();
    db.admin
        .execute(
            "UPDATE case_procedural_fact_revisions SET submission_digest=$3
         WHERE family='resolution' AND id=$1 AND revision=$2",
            &[
                &id.as_uuid(),
                &i64::from(detail.snapshot.metadata().revision.get()),
                &vec![0_u8; 32],
            ],
        )
        .unwrap();
}
