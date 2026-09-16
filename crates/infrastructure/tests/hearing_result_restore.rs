mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;
mod hearing_result_database_support;
mod hearing_result_restore_support;

use application::{cases::*, hearing_results::*, hearings::*, participants::*};
use domain::{crypto::DocumentVersionRef, identity::Role};
use hearing_result_database_support::{persist, service, store, values, Fixture};
use hearing_result_restore_support::{correct, dump_restore, record_at, snapshot, withdraw};
use std::sync::Arc;

#[test]
fn dump_restore_preserves_result_receipts_historical_sources_and_withdrawn_continuation() {
    let Some(mut db) = Fixture::new() else { return };
    hearing_database_support::complete(&mut db);
    let hearing_service = hearing_database_support::service(&db, db.owner, Role::Owner);
    let appointment = hearing_database_support::persist(
        &hearing_service,
        db.case,
        hearing_database_support::schedule(),
    );
    let cancelled = hearing_database_support::persist(
        &hearing_service,
        db.case,
        HearingCommand {
            operation_id: HearingOperationId::new(),
            hearing_id: appointment.snapshot.id,
            change: HearingChange::Cancel {
                expected_revision: appointment.snapshot.revision,
                reason: HearingNote::new("Appointment cancelled independently").unwrap(),
            },
        },
    );
    let participants = infrastructure::PostgresParticipantStore::open(
        &db.runtime_url,
        Arc::new(infrastructure::RingSha256Hasher),
    )
    .unwrap();
    let person = participants
        .create(
            db.owner,
            db.case,
            ParticipantId::new(),
            ParticipantValues::new(
                "Historical witness",
                "Witness",
                None,
                None,
                DirectoryStatus::Active,
            )
            .unwrap(),
            db.at,
        )
        .unwrap();
    let archived = participants
        .change_status(
            db.owner,
            db.case,
            person.id,
            person.revision,
            DirectoryStatus::Archived,
            db.at,
        )
        .unwrap();
    let document = case_stage_database_support::upload(&db, db.case, "declared-record.pdf");
    let agreement_id = HearingResultAgreementId::new();
    let rich = HearingResultValues::new(HearingResultValuesInput {
        occurrence: HearingResultOccurrence::NotStarted,
        extent: HearingResultExtent::Unspecified,
        event_time: values("unused").event_time(),
        summary: HearingResultText::new("Declared attendance without beginning the hearing")
            .unwrap(),
        attendees: vec![HearingResultAttendee::new(
            archived.id(),
            archived.revision_number(),
            HearingResultCapacity::new("Declared witness").unwrap(),
            Some(HearingResultObservation::new("Attendance reported by the operator").unwrap()),
        )],
        agreements: vec![HearingResultAgreement::new(
            agreement_id,
            HearingResultText::new("Follow-up described without an automatic deadline").unwrap(),
        )],
        provenance: HearingResultProvenance::new(
            HearingResultProvenanceKind::WrittenRecord,
            Some(HearingResultReference::new("Record A").unwrap()),
            Some(HearingResultSupportRef::new(
                DocumentVersionRef {
                    id: document.id,
                    version: document.version,
                },
                document.digest,
            )),
        )
        .unwrap(),
    })
    .unwrap();
    let svc = service(&db, db.owner, Role::Owner);
    let first = persist(
        &svc,
        db.case,
        record_at(
            appointment.snapshot.id,
            appointment.snapshot.revision,
            None,
            rich.clone(),
        ),
    );
    let corrected = persist(&svc, db.case, correct(&first, rich));
    let withdrawn = persist(&svc, db.case, withdraw(&corrected));
    let continued = persist(
        &svc,
        db.case,
        record_at(
            appointment.snapshot.id,
            cancelled.snapshot.revision,
            Some(HearingResultContinuationRef::new(
                withdrawn.snapshot.id,
                withdrawn.snapshot.revision,
            )),
            values("A separate declared continuation"),
        ),
    );
    assert_eq!(
        continued.continuation.unwrap().status,
        HearingResultStatus::Withdrawn
    );
    participants
        .change_status(
            db.owner,
            db.case,
            archived.id(),
            archived.revision_number(),
            DirectoryStatus::Active,
            db.at,
        )
        .unwrap();
    let reader = db.user("owner", false);
    db.store()
        .change_administrative_status(
            db.owner,
            db.case,
            CaseRevisionExpectation::new(1),
            CaseAdministrativeStatus::Closed,
            db.at,
        )
        .unwrap();
    db.admin
        .execute(
            "UPDATE users SET active=FALSE,email='former@example.test' WHERE id=$1",
            &[&db.owner.as_uuid()],
        )
        .unwrap();
    let before = snapshot(&mut db);
    db.migrate();
    assert_eq!(snapshot(&mut db), before);
    dump_restore(&mut db);
    assert_eq!(snapshot(&mut db), before);
    let restored = store(&db);
    for exact in [&first, &corrected, &withdrawn, &continued] {
        assert_eq!(
            restored
                .get(
                    reader,
                    db.case,
                    exact.snapshot.hearing_id,
                    exact.snapshot.id,
                    Some(exact.snapshot.revision),
                    db.at,
                )
                .unwrap(),
            *exact
        );
    }
    db.store()
        .change_administrative_status(
            reader,
            db.case,
            CaseRevisionExpectation::new(2),
            CaseAdministrativeStatus::Active,
            db.at,
        )
        .unwrap();
    let next = persist(
        &service(&db, reader, Role::Owner),
        db.case,
        correct(&continued, values("A correction committed after restore")),
    );
    assert_eq!(next.snapshot.revision.get(), 2);
    assert_eq!(next.snapshot.recorded_by.id, reader);
    assert_eq!(next.snapshot.recorded_administration_revision.get(), 3);
    assert_eq!(next.snapshot.anchor, continued.snapshot.anchor);
    assert_eq!(next.continuation, continued.continuation);
}
