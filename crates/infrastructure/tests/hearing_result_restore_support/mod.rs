#![allow(dead_code)]
use super::hearing_result_database_support::Fixture;
use application::hearing_results::*;
use domain::hearings::{HearingId, HearingRevision};

pub fn record_at(
    hearing_id: HearingId,
    anchor_revision: HearingRevision,
    continuation: Option<HearingResultContinuationRef>,
    values: HearingResultValues,
) -> HearingResultCommand {
    HearingResultCommand {
        operation_id: HearingResultOperationId::new(),
        hearing_id,
        result_id: HearingResultId::new(),
        change: HearingResultChange::Record {
            anchor_revision,
            continuation,
            values,
        },
    }
}

pub fn correct(base: &HearingResultDetail, values: HearingResultValues) -> HearingResultCommand {
    HearingResultCommand {
        operation_id: HearingResultOperationId::new(),
        hearing_id: base.snapshot.hearing_id,
        result_id: base.snapshot.id,
        change: HearingResultChange::Correct {
            expected_revision: base.snapshot.revision,
            values,
            reason: HearingResultText::new("Correct declared account").unwrap(),
        },
    }
}

pub fn withdraw(base: &HearingResultDetail) -> HearingResultCommand {
    HearingResultCommand {
        operation_id: HearingResultOperationId::new(),
        hearing_id: base.snapshot.hearing_id,
        result_id: base.snapshot.id,
        change: HearingResultChange::Withdraw {
            expected_revision: base.snapshot.revision,
            reason: HearingResultText::new("Remove this administrative account").unwrap(),
        },
    }
}

pub fn snapshot(db: &mut Fixture) -> serde_json::Value {
    db.admin.query_one(
        "SELECT jsonb_build_object(
         'results',(SELECT jsonb_agg(to_jsonb(h) ORDER BY id) FROM case_hearing_results h),
         'revisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY result_id,revision) FROM case_hearing_result_revisions r),
         'hearings',(SELECT jsonb_agg(to_jsonb(r) ORDER BY hearing_id,revision) FROM case_hearing_revisions r),
         'participants',(SELECT jsonb_agg(to_jsonb(r) ORDER BY participant_id,revision) FROM case_participant_revisions r),
         'documents',(SELECT jsonb_agg(to_jsonb(d) ORDER BY id,version) FROM documents d),
         'administration',(SELECT jsonb_agg(to_jsonb(r) ORDER BY case_id,revision) FROM case_administration_revisions r),
         'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))",
        &[],
    ).unwrap().get(0)
}

pub fn dump_restore(db: &mut Fixture) {
    let directory = tempfile::tempdir().unwrap();
    let dump = directory.path().join("hearing-results.dump");
    run(std::process::Command::new("pg_dump")
        .args([
            "--dbname",
            &db.admin_url,
            "--schema",
            &db.schema,
            "--format=custom",
            "--file",
        ])
        .arg(&dump));
    db.control
        .batch_execute(&format!("DROP SCHEMA {} CASCADE", db.schema))
        .unwrap();
    run(std::process::Command::new("pg_restore")
        .args(["--exit-on-error", "--dbname", &db.admin_url])
        .arg(&dump));
}

fn run(command: &mut std::process::Command) {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
