use super::case_stage_database_support as sources;
use super::hearing_result_database_support::{persist, record, service, values, Fixture};
use super::hearing_result_restore_support::{correct, withdraw};
use super::hearing_result_revalidation_support::*;
use application::{
    documents::{CaseDocumentStore, DocumentRecord, DocumentVersionRef},
    hearing_results::*,
    ApplicationError,
};
use domain::identity::Role;
use infrastructure::PostgresCaseDocumentStore;

fn supported(command: &mut HearingResultCommand, record: &DocumentRecord) {
    let mut selected = input(&values("Declared written source"));
    selected.provenance = HearingResultProvenance::new(
        HearingResultProvenanceKind::WrittenRecord,
        Some(HearingResultReference::new("Written reference A").unwrap()),
        Some(HearingResultSupportRef::new(
            DocumentVersionRef {
                id: record.id,
                version: record.version,
            },
            record.digest,
        )),
    )
    .unwrap();
    set_values(command, HearingResultValues::new(selected).unwrap());
}

#[test]
fn seal_after_preparation_conflicts_but_new_content_does_not_replace_exact_result_support() {
    for seal in [true, false] {
        let Some(mut db) = Fixture::new() else { return };
        let anchor = appointment(&mut db);
        let original = sources::upload(&db, db.case, "minutes.pdf");
        let mut command = record(anchor.snapshot.id);
        supported(&mut command, &original);
        let workflow = service(&db, db.owner, Role::Owner);
        let draft = workflow
            .prepare("session", db.case, command.clone())
            .unwrap();
        let documents = PostgresCaseDocumentStore::open(&db.runtime_url).unwrap();
        let processor = sources::processor();
        let modified = if seal {
            processor.seal(&original).unwrap()
        } else {
            processor
                .prepare_version(
                    original.id,
                    original.version.next().unwrap(),
                    "later.pdf",
                    b"Later exact content",
                )
                .unwrap()
        };
        let (actor, case, at, version) = (db.owner, db.case, db.at, original.version);
        let (interleaved, captured) = watched(&db, actor, Role::Owner, move |_| {
            if seal {
                documents.seal(actor, case, modified.clone(), at).unwrap();
            } else {
                documents
                    .append(actor, case, version, modified.clone(), at)
                    .unwrap();
            }
        });
        let outcome = interleaved.submit("session", case, command, draft.submission_digest);
        if seal {
            assert!(
                matches!(
                    outcome,
                    Err(ApplicationError::HearingResult(
                        HearingResultError::SupportChanged
                    ))
                ),
                "{outcome:?}"
            );
            unchanged(&mut db, captured);
        } else {
            let outcome = outcome.unwrap();
            let support = outcome.support.unwrap();
            assert_eq!(support.reference.version, original.version);
            assert_eq!(support.digest, original.digest);
            assert_eq!(support.name, original.name);
        }
    }
}

#[test]
fn withdrawal_keeps_historical_admission_while_correction_reopens_identical_support() {
    let Some(mut db) = Fixture::new() else { return };
    let anchor = appointment(&mut db);
    let document = sources::upload(&db, db.case, "minutes.pdf");
    let mut command = record(anchor.snapshot.id);
    supported(&mut command, &document);
    let workflow = service(&db, db.owner, Role::Owner);
    let initial = persist(&workflow, db.case, command);
    db.admin
        .batch_execute("ALTER TABLE documents DISABLE TRIGGER USER")
        .unwrap();
    db.admin.execute("UPDATE documents SET vault=set_byte(vault,octet_length(vault)-1,get_byte(vault,octet_length(vault)-1)#1) WHERE id=$1", &[&document.id.as_uuid()]).unwrap();
    db.admin
        .batch_execute("ALTER TABLE documents ENABLE TRIGGER USER")
        .unwrap();
    assert!(workflow
        .prepare(
            "session",
            db.case,
            correct(&initial, initial.snapshot.values.clone())
        )
        .is_err());
    let withdrawn = persist(&workflow, db.case, withdraw(&initial));
    assert_eq!(withdrawn.support, initial.support);
    assert_eq!(withdrawn.snapshot.values, initial.snapshot.values);
    assert_eq!(withdrawn.snapshot.status, HearingResultStatus::Withdrawn);
}
