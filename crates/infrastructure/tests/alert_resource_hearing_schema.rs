use crate::{
    alert_backend_support as alerts, case_administration_support::Fixture,
    deadline_schema_support::open, resource_hearing_database_support as hearings,
};
use postgres::error::SqlState;
use std::sync::Arc;
use time::Duration;
use uuid::Uuid;

const TABLES: [&str; 3] = [
    "alert_subject_state",
    "alert_schedule",
    "alert_notifications",
];

#[test]
fn resource_hearing_alert_columns_are_appended_nullable_and_guarded() {
    let Some(mut db) = Fixture::new() else { return };
    for (table, ordinal) in TABLES.into_iter().zip([8_i16, 14, 17]) {
        let row = db
            .admin
            .query_one(
                "SELECT attnum,format_type(atttypid,atttypmod),attnotnull,
                EXISTS(SELECT 1 FROM pg_attrdef d WHERE d.adrelid=attrelid AND d.adnum=attnum)
             FROM pg_attribute WHERE attrelid=$1::text::regclass AND attname='resource_id'
                AND NOT attisdropped",
                &[&table],
            )
            .expect("resource_id must be appended to each alert projection");
        assert_eq!(row.get::<_, i16>(0), ordinal);
        assert_eq!(row.get::<_, String>(1), "uuid");
        assert!(!row.get::<_, bool>(2));
        assert!(!row.get::<_, bool>(3));
        let mut runtime = db.runtime();
        let allowed: bool = runtime
            .query_one(
                "SELECT has_column_privilege(current_user,$1,'resource_id','SELECT')
                AND has_column_privilege(current_user,$1,'resource_id','INSERT')
                AND NOT has_column_privilege(current_user,$1,'resource_id','UPDATE')",
                &[&table],
            )
            .unwrap()
            .get(0);
        assert!(allowed, "resource parent permissions differ on {table}");
        let error = runtime
            .batch_execute(&format!("UPDATE {table} SET resource_id=resource_id"))
            .unwrap_err();
        assert_eq!(error.code(), Some(&SqlState::INSUFFICIENT_PRIVILEGE));
    }
    open(&db).unwrap();
}

#[test]
fn resource_hearing_alert_migration_preserves_legacy_rows_and_constraint_oids() {
    let Some(mut db) = alerts::fixture() else {
        return;
    };
    alerts::hearing(
        &db,
        db.at.replace_nanosecond(0).unwrap() + Duration::hours(24),
    );
    alerts::accepted_deadline(&db, db.owner);
    let store = alerts::store(&db, Arc::new(alerts::MutableClock::new(db.at)));
    alerts::drive(&store);
    let legacy: i64 = db
        .admin
        .query_one(
            "SELECT count(DISTINCT kind) FROM alert_subject_state WHERE kind IN (0,1)",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(
        legacy, 2,
        "both existing alert families must be represented"
    );
    let rows = alerts::atomicity::snapshot(&mut db);
    let constraints = constraint_inventory(&mut db);
    let business = db.snapshot();
    db.migrate();
    assert_eq!(alerts::atomicity::snapshot(&mut db), rows);
    assert_eq!(constraint_inventory(&mut db), constraints);
    assert_eq!(db.snapshot(), business);
    for table in TABLES {
        let invented: i64 = db
            .admin
            .query_one(
                &format!("SELECT count(*) FROM {table} WHERE kind=2 OR resource_id IS NOT NULL"),
                &[],
            )
            .unwrap()
            .get(0);
        assert_eq!(invented, 0, "migration invented a resource hearing alert");
    }
    let hearings: i64 = db
        .admin
        .query_one("SELECT count(*) FROM case_resource_hearings", &[])
        .unwrap()
        .get(0);
    assert_eq!(hearings, 0);
    open(&db).unwrap();
}

#[test]
fn resource_hearing_alert_shape_and_exact_parent_keys_reject_invalid_rows() {
    let Some(mut db) = Fixture::new() else { return };
    let (_, command) = hearings::setup(&mut db);
    let created = hearings::submit(&db, command);
    let subject = created.origin.hearing_id.as_uuid();
    let resource = created.origin.resource_id.as_uuid();
    for (kind, parent) in [
        (0, Some(resource)),
        (1, Some(resource)),
        (2, None),
        (3, None),
    ] {
        let error = insert_state(&mut db, kind, subject, parent).unwrap_err();
        assert_eq!(error.code(), Some(&SqlState::CHECK_VIOLATION));
    }
    let error = insert_state(&mut db, 2, subject, Some(Uuid::new_v4())).unwrap_err();
    assert_eq!(error.code(), Some(&SqlState::FOREIGN_KEY_VIOLATION));
    let error = insert_state(&mut db, 2, Uuid::new_v4(), Some(resource)).unwrap_err();
    assert_eq!(error.code(), Some(&SqlState::FOREIGN_KEY_VIOLATION));
    insert_state(&mut db, 2, subject, Some(resource)).unwrap();
    let mut schedule = Uuid::new_v4();
    for table in ["alert_schedule", "alert_notifications"] {
        for (kind, parent) in [
            (0, Some(resource)),
            (1, Some(resource)),
            (2, None),
            (3, None),
        ] {
            let error = insert_child(&mut db, table, kind, subject, parent, schedule).unwrap_err();
            assert_eq!(error.code(), Some(&SqlState::CHECK_VIOLATION), "{table}");
        }
        let error =
            insert_child(&mut db, table, 2, subject, Some(Uuid::new_v4()), schedule).unwrap_err();
        assert_eq!(
            error.code(),
            Some(&SqlState::FOREIGN_KEY_VIOLATION),
            "{table}"
        );
        let id = insert_child(&mut db, table, 2, subject, Some(resource), schedule).unwrap();
        if table == "alert_schedule" {
            schedule = id;
        }
    }
    for table in TABLES {
        let error = db
            .admin
            .execute(
                &format!("UPDATE {table} SET resource_id=$1 WHERE kind=2"),
                &[&Uuid::new_v4()],
            )
            .unwrap_err();
        assert_eq!(error.code(), Some(&SqlState::CHECK_VIOLATION), "{table}");
    }
}

#[test]
fn resource_hearing_alert_cursor_accepts_only_the_three_subject_families() {
    let Some(mut db) = Fixture::new() else { return };
    db.runtime()
        .execute(
            "UPDATE alert_scan_cursor SET kind=2,active_kind=2,active_id=$1",
            &[&Uuid::new_v4()],
        )
        .expect("the scanner must be able to resume a resource hearing");
    for change in ["kind=3", "active_kind=3"] {
        let error = db
            .admin
            .batch_execute(&format!("UPDATE alert_scan_cursor SET {change}"))
            .unwrap_err();
        assert_eq!(error.code(), Some(&SqlState::CHECK_VIOLATION));
    }
}

#[test]
fn resource_hearing_alert_startup_requires_parent_constraints_and_scope_key() {
    let Some(mut db) = Fixture::new() else { return };
    for (table, count) in TABLES.into_iter().zip([3, 2, 2]) {
        let rows = db
            .admin
            .query(
                "SELECT c.conname::text,c.contype::text,pg_get_constraintdef(c.oid)
             FROM pg_constraint c JOIN pg_attribute a
                ON a.attrelid=c.conrelid AND a.attnum=ANY(c.conkey)
             WHERE c.conrelid=$1::text::regclass AND a.attname='resource_id'
                AND c.contype IN ('c','f','u') ORDER BY c.conname",
                &[&table],
            )
            .unwrap();
        assert_eq!(
            rows.len(),
            count,
            "missing resource parent constraints on {table}"
        );
        for row in rows {
            let name: String = row.get(0);
            let kind: String = row.get(1);
            let definition: String = row.get(2);
            if kind == "u" {
                assert_eq!(definition, "UNIQUE (kind, id, case_id, resource_id)");
                continue;
            }
            if kind == "f" {
                let expected = if table == "alert_subject_state" {
                    "FOREIGN KEY (id, case_id, resource_id) REFERENCES case_resource_hearings(id, case_id, resource_id)"
                } else {
                    "FOREIGN KEY (kind, subject_id, case_id, resource_id) REFERENCES alert_subject_state(kind, id, case_id, resource_id)"
                };
                assert_eq!(definition, expected);
            }
            db.admin
                .batch_execute(&format!("ALTER TABLE {table} DROP CONSTRAINT {name}"))
                .unwrap();
            assert!(
                open(&db).is_err(),
                "startup accepted missing {table}.{name}"
            );
            db.admin
                .batch_execute(&format!(
                    "ALTER TABLE {table} ADD CONSTRAINT {name} {definition}"
                ))
                .unwrap();
            open(&db).unwrap();
        }
    }
    for table in ["alert_schedule", "alert_notifications"] {
        let retained: bool = db
            .admin
            .query_one(
                "SELECT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid=$1::text::regclass
                AND conname=$2 AND contype='f')",
                &[&table, &format!("{table}_kind_subject_id_fkey")],
            )
            .unwrap()
            .get(0);
        assert!(retained, "the existing subject foreign key must remain");
    }
}

#[test]
fn resource_hearing_alert_startup_rejects_altered_columns_and_parent_grants() {
    let Some(mut db) = Fixture::new() else { return };
    for table in TABLES {
        for (damage, repair) in [
            (format!("ALTER TABLE {table} ALTER COLUMN resource_id SET DEFAULT '00000000-0000-0000-0000-000000000000'::uuid"),
             format!("ALTER TABLE {table} ALTER COLUMN resource_id DROP DEFAULT")),
            (format!("REVOKE INSERT(resource_id) ON {table} FROM {}", db.role),
             format!("GRANT INSERT(resource_id) ON {table} TO {}", db.role)),
            (format!("GRANT UPDATE(resource_id) ON {table} TO {}", db.role),
             format!("REVOKE UPDATE(resource_id) ON {table} FROM {}", db.role)),
        ] {
            db.admin.batch_execute(&damage).unwrap();
            assert!(open(&db).is_err(), "startup accepted {damage}");
            db.admin.batch_execute(&repair).unwrap();
            open(&db).unwrap();
        }
    }
}

fn insert_state(
    db: &mut Fixture,
    kind: i16,
    subject: Uuid,
    resource: Option<Uuid>,
) -> Result<u64, postgres::Error> {
    db.admin.execute(
        "INSERT INTO alert_subject_state(kind,id,case_id,generation,dirty,payload,payload_digest,resource_id)
         VALUES($1,$2,$3,1,false,convert_to('[]','UTF8'),sha256(convert_to('[]','UTF8')),$4)",
        &[&kind, &subject, &db.case.as_uuid(), &resource],
    )
}

fn insert_child(
    db: &mut Fixture,
    table: &str,
    kind: i16,
    subject: Uuid,
    resource: Option<Uuid>,
    schedule: Uuid,
) -> Result<Uuid, postgres::Error> {
    let id = Uuid::new_v4();
    if table == "alert_schedule" {
        db.admin.execute(
            "INSERT INTO alert_schedule(id,kind,subject_id,case_id,recipient,resource_id,
                occurrence_key,occurrence_id,trigger_seconds,trigger_nanos,status,generation,payload,payload_digest)
             VALUES($1,$2,$3,$4,$5,$6,($1::uuid)::text,$1,0,0,'planned',1,
                convert_to('[]','UTF8'),sha256(convert_to('[]','UTF8')))",
            &[&id, &kind, &subject, &db.case.as_uuid(), &db.owner.as_uuid(), &resource],
        )?;
    } else {
        db.admin.execute(
            "INSERT INTO alert_notifications(id,kind,subject_id,case_id,recipient,resource_id,
                schedule_id,created_seconds,created_nanos,internal_enabled,payload,payload_digest)
             VALUES($1,$2,$3,$4,$5,$6,$7,0,0,true,
                convert_to('[]','UTF8'),sha256(convert_to('[]','UTF8')))",
            &[
                &id,
                &kind,
                &subject,
                &db.case.as_uuid(),
                &db.owner.as_uuid(),
                &resource,
                &schedule,
            ],
        )?;
    }
    Ok(id)
}

fn constraint_inventory(db: &mut Fixture) -> serde_json::Value {
    db.admin
        .query_one(
            "SELECT coalesce(jsonb_agg(jsonb_build_array(c.oid,c.conname,
            pg_get_constraintdef(c.oid)) ORDER BY c.oid),'[]'::jsonb)
         FROM pg_constraint c WHERE c.conrelid IN (
            'alert_preferences'::regclass,'alert_subject_state'::regclass,
            'alert_scan_cursor'::regclass,'alert_schedule'::regclass,
            'alert_notifications'::regclass,'alert_read_receipts'::regclass,
            'alert_email_outbox'::regclass,'alert_email_attempts'::regclass)
            AND c.contype<>'t'",
            &[],
        )
        .unwrap()
        .get(0)
}
