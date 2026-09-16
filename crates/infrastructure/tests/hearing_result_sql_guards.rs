mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod hearing_database_support;
mod hearing_result_database_support;
mod hearing_result_sql_support;
use application::hearing_results::*;
use domain::{crypto::Sha256Digest, identity::Role};
use hearing_result_database_support::*;
use hearing_result_sql_support::*;
use serde_json::json;

#[test]
fn direct_sql_rejects_changed_capture_sources_receipt_anchor_and_withdrawal_content() {
    let Some(mut db) = Fixture::new() else { return };
    let first = seed(&mut db);
    let original = source(&mut db, first.snapshot.id, false);
    let command = correction(&first);
    let before = counts(&mut db);
    for (key, value) in [
        (
            "recorded_administration_digest",
            json!(format!("\\x{}", "00".repeat(32))),
        ),
        ("recorded_administration_revision", json!(2)),
        ("recorded_by_email", json!("forged@example.test")),
        ("case_id", json!(domain::cases::CaseId::new().to_string())),
    ] {
        let mut attempted = row(&db, &original, &first, &command, &first.snapshot.values);
        attempted[key] = value;
        reject(&db, &attempted, false);
    }
    let mut forged = first.clone();
    forged.snapshot.anchor.values_digest = Sha256Digest::from_bytes(&[0; 32]).unwrap();
    reject(
        &db,
        &row(&db, &original, &forged, &command, &first.snapshot.values),
        false,
    );
    let withdraw = HearingResultCommand {
        operation_id: HearingResultOperationId::new(),
        hearing_id: first.snapshot.hearing_id,
        result_id: first.snapshot.id,
        change: HearingResultChange::Withdraw {
            expected_revision: first.snapshot.revision,
            reason: HearingResultText::new("Withdraw declaration").unwrap(),
        },
    };
    reject(
        &db,
        &row(
            &db,
            &original,
            &first,
            &withdraw,
            &values("Forged changed content"),
        ),
        false,
    );
    assert_eq!(counts(&mut db), before);
}

#[test]
fn sql_requires_current_authority_contiguous_history_and_terminal_withdrawal() {
    let Some(mut db) = Fixture::new() else { return };
    let first = seed(&mut db);
    let original = source(&mut db, first.snapshot.id, false);
    let mut command = correction(&first);
    if let HearingResultChange::Correct {
        expected_revision, ..
    } = &mut command.change
    {
        *expected_revision = HearingResultRevision::new(3).unwrap();
    }
    reject(
        &db,
        &row(&db, &original, &first, &command, &first.snapshot.values),
        false,
    );
    let attempted = row(
        &db,
        &original,
        &first,
        &correction(&first),
        &first.snapshot.values,
    );
    let mut runtime = db.runtime();
    runtime
        .batch_execute("BEGIN ISOLATION LEVEL REPEATABLE READ")
        .unwrap();
    assert_eq!(
        insert(&mut runtime, &db.schema, &attempted, false)
            .unwrap_err()
            .code()
            .unwrap()
            .code(),
        "23514"
    );
    runtime.batch_execute("ROLLBACK").unwrap();
    db.admin
        .execute(
            "UPDATE users SET active=FALSE WHERE id=$1",
            &[&db.owner.as_uuid()],
        )
        .unwrap();
    reject(&db, &attempted, false);
    db.admin
        .execute(
            "UPDATE users SET active=TRUE WHERE id=$1",
            &[&db.owner.as_uuid()],
        )
        .unwrap();
    let withdrawn = persist(
        &service(&db, db.owner, Role::Owner),
        db.case,
        HearingResultCommand {
            operation_id: HearingResultOperationId::new(),
            hearing_id: first.snapshot.hearing_id,
            result_id: first.snapshot.id,
            change: HearingResultChange::Withdraw {
                expected_revision: first.snapshot.revision,
                reason: HearingResultText::new("Withdraw declaration").unwrap(),
            },
        },
    );
    let original = source(&mut db, first.snapshot.id, false);
    reject(
        &db,
        &row(
            &db,
            &original,
            &withdrawn,
            &correction(&withdrawn),
            &withdrawn.snapshot.values,
        ),
        false,
    );
}

#[test]
fn sql_requires_existing_immutable_anchor_and_continuation_in_the_same_case() {
    let Some(mut db) = Fixture::new() else { return };
    let first = seed(&mut db);
    let original = source(&mut db, first.snapshot.id, true);
    for key in ["anchor_values_digest", "anchor_submission_digest"] {
        let mut root = original.clone();
        root["id"] = json!(HearingResultId::new().to_string());
        root[key] = json!(format!("\\x{}", "00".repeat(32)));
        reject(&db, &root, true);
    }
    let mut root = original.clone();
    root["id"] = json!(HearingResultId::new().to_string());
    root["continuation_hearing_id"] = json!(first.snapshot.hearing_id.to_string());
    root["continuation_result_id"] = json!(first.snapshot.id.to_string());
    root["continuation_revision"] = json!(1);
    root["continuation_values_digest"] =
        json!(format!("\\x{}", first.snapshot.values_digest.to_hex()));
    root["continuation_submission_digest"] = json!(format!(
        "\\x{}",
        first.snapshot.receipt.submission_digest.to_hex()
    ));
    for (key, value) in [
        ("continuation_result_id", root["id"].clone()),
        ("continuation_revision", json!(2)),
        (
            "continuation_hearing_id",
            json!(domain::hearings::HearingId::new().to_string()),
        ),
        (
            "continuation_values_digest",
            json!(format!("\\x{}", "00".repeat(32))),
        ),
        (
            "continuation_submission_digest",
            json!(format!("\\x{}", "00".repeat(32))),
        ),
    ] {
        let mut attempted = root.clone();
        attempted[key] = value;
        reject(&db, &attempted, true);
    }
}

#[test]
fn sql_rejects_future_times_and_unavailable_attendees_or_support() {
    let Some(mut db) = Fixture::new() else { return };
    let first = seed(&mut db);
    let original = source(&mut db, first.snapshot.id, false);
    let command = correction(&first);
    let base = &first.snapshot.values;
    for variant in 0..3 {
        let altered = HearingResultValues::new(HearingResultValuesInput {
            occurrence: base.occurrence(),
            extent: base.extent(),
            event_time: if variant == 0 {
                DeclaredHearingResultTime::instant(
                    (db.at + time::Duration::days(1))
                        .replace_nanosecond(0)
                        .unwrap(),
                )
                .unwrap()
            } else {
                base.event_time()
            },
            summary: base.summary().clone(),
            attendees: if variant == 1 {
                vec![HearingResultAttendee::new(
                    domain::participants::ParticipantId::new(),
                    domain::participants::ParticipantRevision::initial(),
                    HearingResultCapacity::new("Observer").unwrap(),
                    None,
                )]
            } else {
                vec![]
            },
            agreements: vec![],
            provenance: if variant == 2 {
                HearingResultProvenance::new(
                    HearingResultProvenanceKind::OperatorNote,
                    None,
                    Some(HearingResultSupportRef::new(
                        application::documents::DocumentVersionRef {
                            id: domain::crypto::DocumentId::new(),
                            version: domain::crypto::DocumentVersion::initial(),
                        },
                        Sha256Digest::from_bytes(&[1; 32]).unwrap(),
                    )),
                )
                .unwrap()
            } else {
                base.provenance().clone()
            },
        })
        .unwrap();
        let mut attempted = row(&db, &original, &first, &command, &altered);
        if variant == 2 {
            attempted["support_name"] = json!("missing.pdf");
            attempted["support_format"] = json!("pdf");
            attempted["support_policy"] = json!("pdf_docx_v1");
        }
        reject(&db, &attempted, false);
    }
}
