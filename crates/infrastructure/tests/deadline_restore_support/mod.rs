use crate::case_administration_support::Fixture;
use serde_json::Value;

pub fn inventory_snapshot(db: &mut Fixture) -> Value {
    db.admin.query_one("SELECT jsonb_build_object(
        'roots',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM case_deadlines r),
        'revisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY deadline_id,revision) FROM case_deadline_revisions r),
        'profiles',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM deadline_profiles r),
        'profile_revisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY profile_id,revision) FROM deadline_profile_revisions r),
        'facts',(SELECT jsonb_agg(to_jsonb(r) ORDER BY family,id) FROM case_procedural_facts r),
        'fact_revisions',(SELECT jsonb_agg(to_jsonb(r) ORDER BY family,id,revision) FROM case_procedural_fact_revisions r),
        'cases',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM cases r),
        'administration',(SELECT jsonb_agg(to_jsonb(r) ORDER BY case_id,revision) FROM case_administration_revisions r),
        'memberships',(SELECT jsonb_agg(to_jsonb(r) ORDER BY case_id,user_id) FROM case_memberships r),
        'users',(SELECT jsonb_agg(jsonb_build_object('id',id,'email',email,'role',role,'active',active) ORDER BY id) FROM users),
        'events',(SELECT jsonb_agg(to_jsonb(r) ORDER BY sequence) FROM deadline_source_events r),
        'event_sequence',(SELECT jsonb_build_object('last_value',last_value,'is_called',is_called) FROM deadline_source_events_sequence),
        'audit',(SELECT jsonb_agg(to_jsonb(r) ORDER BY sequence) FROM audit_events r))", &[]).unwrap().get(0)
}
pub fn dump_restore(db: &mut Fixture) {
    let directory = tempfile::tempdir().unwrap();
    let dump = directory.path().join("deadlines.dump");
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

pub fn expressions(db: &mut Fixture) -> std::collections::BTreeMap<String, String> {
    db.admin.query("SELECT c.relname||'.'||k.conname AS key,pg_get_expr(k.conbin,k.conrelid) AS expression FROM pg_constraint k JOIN pg_class c ON c.oid=k.conrelid WHERE k.conrelid IN ('case_deadlines'::regclass,'case_deadline_revisions'::regclass) AND k.contype='c'
        UNION ALL SELECT c.relname||'.'||a.attname,pg_get_expr(d.adbin,d.adrelid) FROM pg_attrdef d JOIN pg_class c ON c.oid=d.adrelid JOIN pg_attribute a ON a.attrelid=d.adrelid AND a.attnum=d.adnum WHERE d.adrelid IN ('case_deadlines'::regclass,'case_deadline_revisions'::regclass)", &[]).unwrap().into_iter().map(|row| (row.get(0),row.get(1))).collect()
}
