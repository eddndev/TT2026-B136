use super::Fixture;

pub fn ledger(db: &mut Fixture) -> serde_json::Value {
    db.admin.query_one("SELECT jsonb_build_object(
        'jobs',(SELECT jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text) FROM case_report_jobs r),
        'captures',(SELECT jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text) FROM case_report_snapshots r),
        'artifacts',(SELECT jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text) FROM case_report_artifacts r),
        'notices',(SELECT jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text) FROM case_report_notices r),
        'audit',(SELECT jsonb_agg(to_jsonb(r) ORDER BY sequence) FROM audit_events r))", &[]).unwrap().get(0)
}
pub fn reject_audit(db: &mut Fixture, action: &str, prerequisite: &str) {
    db.admin.batch_execute(&format!("CREATE SEQUENCE case_report_fault_reached;
        GRANT USAGE ON SEQUENCE case_report_fault_reached TO {};
        CREATE FUNCTION reject_case_report_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
        IF NEW.action <> '{action}' THEN RETURN NEW; END IF;
        IF NOT ({prerequisite}) THEN RAISE EXCEPTION 'report audit preceded required writes'; END IF;
        PERFORM nextval('case_report_fault_reached');
        RAISE EXCEPTION 'injected report audit failure after writes';
        END; $$;
        CREATE TRIGGER reject_case_report_audit BEFORE INSERT ON audit_events
        FOR EACH ROW EXECUTE FUNCTION reject_case_report_audit()", db.role)).unwrap();
}
pub fn fault_reached(db: &mut Fixture) -> bool {
    db.admin
        .query_one("SELECT is_called FROM case_report_fault_reached", &[])
        .unwrap()
        .get(0)
}
pub fn allow_audit(db: &mut Fixture) {
    db.admin
        .batch_execute(
            "DROP TRIGGER reject_case_report_audit ON audit_events;
        DROP FUNCTION reject_case_report_audit(); DROP SEQUENCE case_report_fault_reached",
        )
        .unwrap();
}
