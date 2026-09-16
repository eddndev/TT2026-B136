#[allow(dead_code)]
mod legacy_database_support;
use application::judicial_calendars::*;
use domain::{crypto::DocumentHasher, identity::UserId};
use infrastructure::RingSha256Hasher;
use legacy_database_support::{Database, Source};
use serde_json::Value;
use uuid::Uuid;

#[test]
fn initial_import_rejects_calendar_roots_or_orphan_revisions_without_other_history() {
    for root_only in [true, false] {
        let Some(mut db) = Database::new() else {
            return;
        };
        let source = Source::new();
        db.seed_case(source.case_id);
        let id = Uuid::new_v4();
        if root_only {
            db.client
                .batch_execute("ALTER TABLE judicial_calendars DISABLE TRIGGER ALL")
                .unwrap();
            db.client
                .execute("INSERT INTO judicial_calendars(id) VALUES($1)", &[&id])
                .unwrap();
            db.client
                .batch_execute("ALTER TABLE judicial_calendars ENABLE TRIGGER ALL")
                .unwrap();
        } else {
            let fixtures: Vec<Value> = serde_json::from_str(include_str!(
                "../../domain/tests/fixtures/judicial_calendar_vectors.json"
            ))
            .unwrap();
            let encoded = fixtures[0]["hex"].as_str().unwrap();
            let bytes = (0..encoded.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&encoded[i..i + 2], 16).unwrap())
                .collect::<Vec<_>>();
            let values = JudicialCalendarValues::from_canonical_bytes(&bytes).unwrap();
            let actor = db
                .client
                .query_one("SELECT id,email FROM users LIMIT 1", &[])
                .unwrap();
            let actor_id: Uuid = actor.get(0);
            let email: String = actor.get(1);
            let command = JudicialCalendarCommand {
                operation_id: JudicialCalendarOperationId::new(),
                calendar_id: JudicialCalendarId::from_uuid(id),
                change: JudicialCalendarChange::Publish { values },
            };
            let digest = RingSha256Hasher.hash_bytes(&bytes);
            let submission =
                judicial_calendar_submission_bytes(UserId::from_uuid(actor_id), &command, digest);
            db.client
                .batch_execute("ALTER TABLE judicial_calendar_revisions DISABLE TRIGGER ALL")
                .unwrap();
            db.client.execute("INSERT INTO judicial_calendar_revisions(calendar_id,revision,values_canonical,values_digest,operation_id,action,submission_canonical,submission_digest,recorded_at_seconds,recorded_at_nanoseconds,recorded_by,recorded_by_email)
                VALUES($1,1,$2,sha256($2),$3,'publish',$4,sha256($4),0,0,$5,$6)",&[&id,&bytes,&command.operation_id.as_uuid(),&submission,&actor_id,&email]).unwrap();
            db.client
                .batch_execute("ALTER TABLE judicial_calendar_revisions ENABLE TRIGGER ALL")
                .unwrap();
        }
        let before = db.stored_state();
        let calendars=db.client.query_one("SELECT (SELECT count(*) FROM judicial_calendars),(SELECT count(*) FROM judicial_calendar_revisions)",&[]).unwrap();
        assert!(source.inspect().check_target(&db.url).is_err());
        assert!(source.inspect().apply(&db.url).is_err());
        assert_eq!(db.stored_state(), before);
        let after=db.client.query_one("SELECT (SELECT count(*) FROM judicial_calendars),(SELECT count(*) FROM judicial_calendar_revisions)",&[]).unwrap();
        assert_eq!(
            (after.get::<_, i64>(0), after.get::<_, i64>(1)),
            (calendars.get::<_, i64>(0), calendars.get::<_, i64>(1))
        );
        source.assert_unmarked();
    }
}
