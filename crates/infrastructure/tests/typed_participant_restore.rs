mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
mod declaration_fixture;
mod typed_participant_service_support;

use application::{
    credential_trust::{CredentialTrustExpectation, CredentialTrustStore},
    participants::{ParticipantStore, ParticipantValues},
    typed_participants::*,
};
use domain::crypto::{InternalDeclarationVerifier, Signature};
use infrastructure::{
    certificates::InternalRsaDeclarationVerifier, PostgresCredentialTrustStore,
    PostgresParticipantStore, RingSha256Hasher,
};
use std::{process::Command, sync::Arc};
use typed_participant_service_support::*;

#[test]
fn restore_preserves_signed_union_history_exact_subject_and_all_public_evidence_bytes() {
    let Some(mut db) = Fixture::new() else { return };
    let fx = declaration_fixture::fixture();
    db.at = time::OffsetDateTime::from_unix_timestamp(fx.at)
        .unwrap()
        .replace_nanosecond(123456789)
        .unwrap();
    let trust = InternalRsaDeclarationVerifier
        .inspect_trust(&fx.root, &fx.crl, fx.at)
        .unwrap();
    let trust_store =
        PostgresCredentialTrustStore::open(&db.admin_url, Arc::new(FixedClock(db.at))).unwrap();
    trust_store
        .publish(CredentialTrustExpectation::Absent, trust)
        .unwrap();
    drop(trust_store);
    let record = upload(&db, db.case, "identity.pdf");
    let manual_store = manual(&db);
    let original_manual = manual_store
        .create(
            db.owner,
            db.case,
            ParticipantId::new(),
            ParticipantValues::new(
                "Manual declaration",
                "Court",
                None,
                None,
                DirectoryStatus::Active,
            )
            .unwrap(),
            db.at,
        )
        .unwrap();
    let workflow = service(&db, FormatCheck(None));
    let mut value = proposal(&record);
    value.participant = ParticipantDraftTarget::Existing {
        id: original_manual.id,
        expected_revision: original_manual.revision,
    };
    value.certificate = Some(fx.leaf.clone());
    value.role = ParticipantRoleValues::new(
        None,
        None,
        ParticipantProfile::ControlJudge(ControlJudgeProfile::new("Declared court").unwrap()),
        value.role.role_support().clone(),
    )
    .unwrap();
    let mut request = reviewed(
        workflow
            .review_participant("session", db.case, value)
            .unwrap(),
    );
    request.certificate = Some(fx.leaf.clone());
    let declaration = workflow
        .prepare_participant("session", db.case, request.clone())
        .unwrap()
        .declaration
        .unwrap();
    assert_eq!(declaration.bytes.len(), 218);
    let directory = tempfile::tempdir().unwrap();
    let statement = directory.path().join("statement.bin");
    std::fs::write(&statement, &declaration.bytes).unwrap();
    let signature = Signature::from_bytes(declaration_fixture::openssl(&[
        "dgst",
        "-sha256",
        "-sign",
        fx.ca
            .join("private/synthetic-declarant.key.pem")
            .to_str()
            .unwrap(),
        statement.to_str().unwrap(),
    ]))
    .unwrap();
    let committed = workflow
        .submit_participant(
            "session",
            db.case,
            ParticipantSubmission {
                prepared: request,
                signature: Some(signature),
            },
        )
        .unwrap();
    let original_subject = committed.bound_subject.clone().unwrap();
    let ParticipantRevisionSnapshot::Typed(ref typed) = committed.revision else {
        panic!("typed revision required")
    };
    assert_eq!(typed.revision.get(), 2);
    let original_evidence = store(&db)
        .credential(db.owner, db.case, typed.id, typed.revision, db.at)
        .unwrap();
    let archived = manual_store
        .change_status(
            db.owner,
            db.case,
            typed.id,
            typed.revision,
            DirectoryStatus::Archived,
            db.at,
        )
        .unwrap();
    assert_eq!(archived.revision_number().get(), 3);
    let current_subject = replace_subject(&workflow, &db, &original_subject);
    assert_eq!(current_subject.revision.get(), 2);
    drop(workflow);
    drop(manual_store);

    let before = all_rows(&mut db);
    let dump = directory.path().join("signed-participants.dump");
    success(
        Command::new("pg_dump")
            .args([
                "--dbname",
                &db.admin_url,
                "--schema",
                &db.schema,
                "--format=custom",
                "--file",
            ])
            .arg(&dump),
    );
    db.control
        .batch_execute(&format!("DROP SCHEMA {} CASCADE", db.schema))
        .unwrap();
    success(
        Command::new("pg_restore")
            .args(["--exit-on-error", "--dbname", &db.admin_url])
            .arg(&dump),
    );
    assert_eq!(all_rows(&mut db), before);
    db.migrate();
    assert_eq!(all_rows(&mut db), before);

    let restored = store(&db);
    let restored_manual = manual(&db);
    let history = restored_manual
        .history(
            db.owner,
            db.case,
            typed.id,
            application::participants::ParticipantHistoryQuery::new(10, None).unwrap(),
            db.at,
        )
        .unwrap();
    assert_eq!(history.revisions.len(), 3);
    assert_eq!(history.revisions[0], archived);
    assert_eq!(history.revisions[1], committed);
    assert_eq!(history.revisions[2].manual().unwrap(), &original_manual);
    assert_eq!(
        restored
            .get_subject(db.owner, db.case, original_subject.id, db.at)
            .unwrap(),
        current_subject
    );
    assert_eq!(
        restored
            .get_subject_revision(
                db.owner,
                db.case,
                original_subject.id,
                original_subject.revision,
                db.at,
            )
            .unwrap(),
        original_subject
    );
    let evidence = restored
        .credential(
            db.owner,
            db.case,
            typed.id,
            archived.revision_number(),
            db.at,
        )
        .unwrap();
    assert_eq!(evidence, original_evidence);
    assert_eq!(evidence.declaration, declaration.bytes);
    assert_eq!(evidence.accepted_at.nanosecond(), 123456789);
    assert_eq!(evidence.accepted_by.email, "owner@example.test");
    let reactivated = restored_manual
        .change_status(
            db.owner,
            db.case,
            typed.id,
            archived.revision_number(),
            DirectoryStatus::Active,
            db.at,
        )
        .unwrap();
    assert_eq!(reactivated.revision_number().get(), 4);
    assert_eq!(
        store(&db)
            .credential(
                db.owner,
                db.case,
                typed.id,
                reactivated.revision_number(),
                db.at,
            )
            .unwrap(),
        evidence
    );
}

fn replace_subject(
    workflow: &TypedParticipantService,
    db: &Fixture,
    original: &SubjectSnapshot,
) -> SubjectSnapshot {
    let values = SubjectValues::natural_person(
        RepresentedName::Known(ParticipantText::new("Ana Updated").unwrap()),
        Declared::Unknown(ParticipantReason::new("Not declared").unwrap()),
        original.values.identity_support().clone(),
    );
    let review = workflow
        .review_subject(
            "session",
            db.case,
            original.id,
            original.revision,
            values.clone(),
        )
        .unwrap();
    assert!(review.candidates.is_empty());
    workflow
        .replace_subject(
            "session",
            db.case,
            SubjectReplacementRequest {
                id: original.id,
                expected_revision: original.revision,
                values,
                review: IdentityReviewSubmission {
                    directory_stamp: review.directory_stamp,
                    different: vec![],
                    selection_reason: ParticipantReason::new("Updated identity reviewed").unwrap(),
                },
            },
        )
        .unwrap()
}

fn manual(db: &Fixture) -> PostgresParticipantStore {
    PostgresParticipantStore::open(&db.runtime_url, Arc::new(RingSha256Hasher)).unwrap()
}

fn all_rows(db: &mut Fixture) -> serde_json::Value {
    let mut result = serde_json::Map::new();
    for row in db
        .admin
        .query(
            "SELECT tablename FROM pg_catalog.pg_tables WHERE schemaname=$1 ORDER BY tablename",
            &[&db.schema],
        )
        .unwrap()
    {
        let table: String = row.get(0);
        let quoted = table.replace('"', "\"\"");
        let rows = db
            .admin
            .query_one(
                &format!(
                    "SELECT COALESCE(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text),'[]') \
                     FROM \"{quoted}\" t"
                ),
                &[],
            )
            .unwrap()
            .get(0);
        result.insert(table, rows);
    }
    serde_json::Value::Object(result)
}

fn success(command: &mut Command) {
    let result = command.output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}
