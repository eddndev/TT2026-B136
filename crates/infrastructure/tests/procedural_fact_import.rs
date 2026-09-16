#[allow(dead_code)]
mod legacy_database_support;
use application::procedural_facts::*;
use domain::{crypto::DocumentHasher, identity::UserId};
use infrastructure::RingSha256Hasher;
use legacy_database_support::{Database, Source};
use serde_json::Value;
use uuid::Uuid;

#[test]
fn initial_import_rejects_either_fact_family_roots_or_orphan_revisions() {
    for family in ["resolution", "notification"] {
        for root_only in [true, false] {
            let Some(mut db) = Database::new() else {
                return;
            };
            let source = Source::new();
            db.seed_case(source.case_id);
            let id = Uuid::new_v4();
            let parent = Uuid::new_v4();
            if root_only {
                db.client
                    .batch_execute("ALTER TABLE case_procedural_facts DISABLE TRIGGER ALL")
                    .unwrap();
                let parent = if family == "notification" {
                    Some(parent)
                } else {
                    None
                };
                db.client.execute("INSERT INTO case_procedural_facts(family,id,case_id,parent_resolution_id) VALUES($1,$2,$3,$4)",&[&family,&id,&source.case_id.as_uuid(),&parent]).unwrap();
                db.client
                    .batch_execute("ALTER TABLE case_procedural_facts ENABLE TRIGGER ALL")
                    .unwrap();
            } else {
                let fixtures: Vec<Value> = serde_json::from_str(include_str!(
                    "../../domain/tests/fixtures/procedural_fact_vectors.json"
                ))
                .unwrap();
                let v = fixtures
                    .iter()
                    .find(|v| v["name"] == format!("{family}_minimum"))
                    .unwrap();
                let hex = v["hex"].as_str().unwrap();
                let bytes = (0..hex.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                    .collect::<Vec<_>>();
                let values =
                    infrastructure::procedural_fact_codec::values(family, &bytes, &v["normalized"])
                        .unwrap();
                let op = FactOperationId::new();
                let command = match values {
                    ProceduralFactValues::Resolution(v) => {
                        ProceduralFactCommand::Resolution(ResolutionCommand::new(
                            op,
                            ResolutionId::from_uuid(id),
                            FactChange::record(*v),
                        ))
                    }
                    ProceduralFactValues::Notification(v) => ProceduralFactCommand::Notification(
                        NotificationCommand::new(
                            op,
                            NotificationId::from_uuid(id),
                            v.resolution().id,
                            FactChange::record(*v),
                        )
                        .unwrap(),
                    ),
                };
                let actor = db
                    .client
                    .query_one("SELECT id,email FROM users LIMIT 1", &[])
                    .unwrap();
                let actor_id: Uuid = actor.get(0);
                let email: String = actor.get(1);
                let baseline = db
                    .client
                    .query_one(
                        "SELECT title,reference FROM cases WHERE id=$1",
                        &[&source.case_id.as_uuid()],
                    )
                    .unwrap();
                let title: String = baseline.get(0);
                let reference: String = baseline.get(1);
                let mut sources = b"PFSRC1".to_vec();
                sources.extend_from_slice(&[0; 13]);
                let submission = fact_submission_bytes(
                    UserId::from_uuid(actor_id),
                    source.case_id,
                    &command,
                    RingSha256Hasher.hash_bytes(&bytes),
                    RingSha256Hasher.hash_bytes(&sources),
                )
                .unwrap();
                db.client
                    .batch_execute("ALTER TABLE case_procedural_fact_revisions DISABLE TRIGGER ALL")
                    .unwrap();
                db.client.execute("INSERT INTO case_procedural_fact_revisions(family,id,case_id,revision,values_canonical,values_digest,sources_canonical,sources_digest,operation_id,action,submission_canonical,submission_digest,recorded_administration_title,recorded_administration_reference,recorded_at_seconds,recorded_at_nanoseconds,recorded_by,recorded_by_email) VALUES($1,$2,$3,1,$4,sha256($4),$5,sha256($5),$6,'record',$7,sha256($7),$8,$9,0,0,$10,$11)",&[&family,&id,&source.case_id.as_uuid(),&bytes,&sources,&op.as_uuid(),&submission,&title,&reference,&actor_id,&email]).unwrap();
                db.client
                    .batch_execute("ALTER TABLE case_procedural_fact_revisions ENABLE TRIGGER ALL")
                    .unwrap();
            }
            let before = db.stored_state();
            let facts = counts(&mut db);
            assert!(
                source.inspect().check_target(&db.url).is_err(),
                "{family}; root_only={root_only}"
            );
            assert!(source.inspect().apply(&db.url).is_err());
            assert_eq!(db.stored_state(), before);
            assert_eq!(counts(&mut db), facts);
            source.assert_unmarked();
        }
    }
}
fn counts(db: &mut Database) -> (i64, i64) {
    let row=db.client.query_one("SELECT (SELECT count(*) FROM case_procedural_facts),(SELECT count(*) FROM case_procedural_fact_revisions)",&[]).unwrap();
    (row.get(0), row.get(1))
}
