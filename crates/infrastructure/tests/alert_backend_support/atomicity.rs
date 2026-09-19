use super::Fixture;

pub fn snapshot(db: &mut Fixture) -> serde_json::Value {
    db.admin
        .query_one(
            "SELECT jsonb_build_object(
        'preferences',(SELECT jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text)
            FROM alert_preferences r),
        'subjects',(SELECT jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text)
            FROM alert_subject_state r),
        'schedule',(SELECT jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text)
            FROM alert_schedule r),
        'notifications',(SELECT jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text)
            FROM alert_notifications r),
        'outbox',(SELECT jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text)
            FROM alert_email_outbox r),
        'cursor',(SELECT jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r)::text)
            FROM alert_scan_cursor r),
        'audit',(SELECT jsonb_agg(to_jsonb(r) ORDER BY sequence) FROM audit_events r))",
            &[],
        )
        .unwrap()
        .get(0)
}

pub fn reject_activation_audit_after_writes(db: &mut Fixture) {
    db.admin
        .batch_execute(&format!(
            "CREATE SEQUENCE alert_fault_reached;
        GRANT USAGE ON SEQUENCE alert_fault_reached TO {};
        CREATE FUNCTION reject_alert_activation_audit() RETURNS trigger
        LANGUAGE plpgsql AS $$ BEGIN
        IF NOT EXISTS(SELECT 1 FROM alert_notifications) OR
           NOT EXISTS(SELECT 1 FROM alert_email_outbox) THEN
            RAISE EXCEPTION 'alert audit preceded notification and outbox'; END IF;
        PERFORM nextval('alert_fault_reached');
        RAISE EXCEPTION 'injected alert audit failure after activation writes';
        END; $$;
        CREATE TRIGGER reject_alert_activation_audit BEFORE INSERT ON audit_events
        FOR EACH ROW EXECUTE FUNCTION reject_alert_activation_audit()",
            db.role
        ))
        .unwrap();
}

pub fn fault_reached(db: &mut Fixture) -> bool {
    db.admin
        .query_one("SELECT is_called FROM alert_fault_reached", &[])
        .unwrap()
        .get(0)
}

pub fn allow_activation_audit(db: &mut Fixture) {
    db.admin
        .batch_execute(
            "DROP TRIGGER reject_alert_activation_audit ON audit_events;
        DROP FUNCTION reject_alert_activation_audit(); DROP SEQUENCE alert_fault_reached",
        )
        .unwrap();
}

pub fn reject_scan_audit_after_writes(db: &mut Fixture) {
    db.admin
        .batch_execute(&format!(
            "CREATE SEQUENCE alert_scan_fault_reached;
        GRANT USAGE ON SEQUENCE alert_scan_fault_reached TO {};
        CREATE FUNCTION reject_alert_scan_audit() RETURNS trigger
        LANGUAGE plpgsql AS $$ BEGIN
        IF NOT EXISTS(SELECT 1 FROM alert_schedule) THEN
            RAISE EXCEPTION 'scan audit preceded durable schedule'; END IF;
        PERFORM nextval('alert_scan_fault_reached');
        RAISE EXCEPTION 'injected alert audit failure after scan writes';
        END; $$;
        CREATE TRIGGER reject_alert_scan_audit BEFORE INSERT ON audit_events
        FOR EACH ROW EXECUTE FUNCTION reject_alert_scan_audit()",
            db.role
        ))
        .unwrap();
}

pub fn scan_fault_reached(db: &mut Fixture) -> bool {
    db.admin
        .query_one("SELECT is_called FROM alert_scan_fault_reached", &[])
        .unwrap()
        .get(0)
}

pub fn allow_scan_audit(db: &mut Fixture) {
    db.admin
        .batch_execute(
            "DROP TRIGGER reject_alert_scan_audit ON audit_events;
        DROP FUNCTION reject_alert_scan_audit(); DROP SEQUENCE alert_scan_fault_reached",
        )
        .unwrap();
}
