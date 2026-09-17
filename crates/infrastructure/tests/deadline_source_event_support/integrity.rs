use super::Fixture;
use postgres::{error::SqlState, types::ToSql};
use uuid::Uuid;

#[derive(Clone)]
pub struct Event {
    pub kind: String,
    pub id: Uuid,
    pub revision: i64,
    pub case: Option<Uuid>,
    pub hearing: Option<Uuid>,
    pub operation: Uuid,
}

pub fn event(db: &mut Fixture, kind: &str) -> Event {
    let row = db.admin.query_one("SELECT source_kind,source_id,revision,case_id,hearing_id,operation_id FROM deadline_source_events WHERE source_kind=$1 ORDER BY sequence LIMIT 1", &[&kind]).unwrap();
    Event {
        kind: row.get(0),
        id: row.get(1),
        revision: row.get(2),
        case: row.get(3),
        hearing: row.get(4),
        operation: row.get(5),
    }
}

pub fn insert(db: &mut Fixture, event: &Event) -> Result<u64, postgres::Error> {
    db.admin.execute("INSERT INTO deadline_source_events(source_kind,source_id,revision,case_id,hearing_id,operation_id) VALUES($1,$2,$3,$4,$5,$6)",
        &[&event.kind,&event.id,&event.revision,&event.case,&event.hearing,&event.operation])
}

pub fn remove_event_temporarily(db: &mut Fixture, event: &Event) {
    db.admin
        .batch_execute("BEGIN; ALTER TABLE deadline_source_events DISABLE TRIGGER USER")
        .unwrap();
    assert_eq!(db.admin.execute("DELETE FROM deadline_source_events WHERE source_kind=$1 AND source_id=$2 AND revision=$3", &[&event.kind,&event.id,&event.revision]).unwrap(), 1);
    db.admin
        .batch_execute("ALTER TABLE deadline_source_events ENABLE TRIGGER USER")
        .unwrap();
}

pub fn rejects_event(db: &mut Fixture, event: &Event) {
    db.admin.batch_execute("SAVEPOINT rejected_event").unwrap();
    let error = insert(db, event).unwrap_err();
    assert!(
        matches!(error.code(), Some(code) if *code == SqlState::CHECK_VIOLATION || *code == SqlState::FOREIGN_KEY_VIOLATION || *code == SqlState::NOT_NULL_VIOLATION),
        "unexpected event rejection: {error:?}"
    );
    db.admin
        .batch_execute("ROLLBACK TO SAVEPOINT rejected_event; RELEASE SAVEPOINT rejected_event")
        .unwrap();
}

pub fn replay_revision(db: &mut Fixture, event: &Event) {
    let (table, id_column) = match event.kind.as_str() {
        "resolution" | "notification" => ("case_procedural_fact_revisions", "id"),
        "hearing_result" => ("case_hearing_result_revisions", "result_id"),
        "calendar" => ("judicial_calendar_revisions", "calendar_id"),
        _ => panic!("unknown source family"),
    };
    let columns: String = db.admin.query_one("SELECT string_agg(quote_ident(attname),',' ORDER BY attnum) FROM pg_attribute WHERE attrelid=$1::text::regclass AND attnum>0 AND NOT attisdropped AND attgenerated=''", &[&table]).unwrap().get(0);
    let family = matches!(event.kind.as_str(), "resolution" | "notification");
    let predicate = format!(
        "{id_column}=$1 AND revision=$2{}",
        if family { " AND family=$3" } else { "" }
    );
    let mut params: Vec<&(dyn ToSql + Sync)> = vec![&event.id, &event.revision];
    if family {
        params.push(&event.kind);
    }
    db.admin.execute(&format!("CREATE TEMP TABLE replay_source AS SELECT {columns} FROM {table} WHERE {predicate}"), &params).unwrap();
    // Deletion only stages a SQL replay; reenable every guard before the tested insert.
    db.admin
        .batch_execute(&format!("ALTER TABLE {table} DISABLE TRIGGER ALL"))
        .unwrap();
    assert_eq!(
        db.admin
            .execute(&format!("DELETE FROM {table} WHERE {predicate}"), &params)
            .unwrap(),
        1
    );
    db.admin
        .batch_execute(&format!("ALTER TABLE {table} ENABLE TRIGGER ALL"))
        .unwrap();
    assert_eq!(
        db.admin
            .execute(
                &format!("INSERT INTO {table}({columns}) SELECT {columns} FROM replay_source"),
                &[]
            )
            .unwrap(),
        1
    );
}
