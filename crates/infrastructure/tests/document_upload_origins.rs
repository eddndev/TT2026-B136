use crate::metadata_database_support;
use application::documents::{CaseDocumentStore, DocumentRecord, MetadataRevision};
use application::ApplicationError;
use domain::audit::{chain_digest, AuditEvent, GENESIS_PREVIOUS};
use domain::cases::CaseId;
use domain::crypto::DocumentId;
use domain::identity::UserId;
use infrastructure::{PostgresCaseDocumentStore, RingSha256Hasher};
use metadata_database_support::{document, metadata, version_database_support::Database, Fixture};
use serde_json::Value;
use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};

fn litigator(f: &mut Fixture) -> UserId {
    let id = UserId::new();
    f.db.client
        .execute(
            "INSERT INTO users(id,email,password_hash,role,protected_totp_secret,recovery_codes)
             VALUES($1,$2,'fixture','litigator','\\x00','{}')",
            &[&id.as_uuid(), &format!("{id}@example.test")],
        )
        .unwrap();
    f.db.client
        .execute(
            "INSERT INTO case_memberships(case_id,user_id) VALUES($1,$2)",
            &[&f.case.as_uuid(), &id.as_uuid()],
        )
        .unwrap();
    id
}

fn origin(f: &mut Fixture, id: DocumentId) -> Value {
    let rows =
        f.db.client
            .query(
                "SELECT to_jsonb(o) FROM document_upload_origins o WHERE document_id=$1",
                &[&id.as_uuid()],
            )
            .unwrap();
    assert_eq!(rows.len(), 1, "one original upload has exactly one origin");
    rows[0].get(0)
}

fn checked_origin(
    f: &mut Fixture,
    record: &DocumentRecord,
    actor: UserId,
    at: OffsetDateTime,
) -> Value {
    let value = origin(f, record.id);
    let email = format!("{actor}@example.test");
    assert_eq!(value["document_id"], record.id.to_string());
    assert_eq!(value["case_id"], f.case.to_string());
    assert_eq!(value["actor_id"], actor.to_string());
    assert_eq!(value["actor_email"], email);
    assert_eq!(value["recorded_at_seconds"], at.unix_timestamp());
    assert_eq!(value["recorded_at_nanoseconds"], at.nanosecond());
    let sequence = value["audit_sequence"].as_i64().unwrap();
    let audit =
        f.db.client
            .query_one(
                "SELECT actor,action,resource,timestamp FROM audit_events WHERE sequence=$1",
                &[&sequence],
            )
            .unwrap();
    assert_eq!(audit.get::<_, String>("actor"), email);
    assert_eq!(audit.get::<_, String>("action"), "document.uploaded");
    assert_eq!(
        audit.get::<_, String>("resource"),
        format!(
            "case:{}:document:{}:version:1:sha256:{}",
            f.case,
            record.id,
            record.digest.to_hex()
        )
    );
    assert_eq!(
        audit.get::<_, String>("timestamp"),
        at.format(&Rfc3339).unwrap()
    );
    value
}

fn snapshot(f: &mut Fixture) -> Value {
    let mut value = f.snapshot();
    value["upload_origins"] =
        f.db.client
            .query_one(
                "SELECT coalesce(jsonb_agg(to_jsonb(o) ORDER BY document_id),'[]'::jsonb)
             FROM document_upload_origins o",
                &[],
            )
            .unwrap()
            .get(0);
    value
}

#[test]
fn original_upload_author_survives_reassignment_append_and_classification() {
    let Some(mut f) = Fixture::new() else { return };
    let uploader = litigator(&mut f);
    let replacement = litigator(&mut f);
    let store = f.store();
    let first = document();
    let at = f.at;
    store.insert(uploader, f.case, first.clone(), at).unwrap();
    let saved = checked_origin(&mut f, &first, uploader, at);
    f.db.client
        .execute(
            "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
            &[&f.case.as_uuid(), &uploader.as_uuid()],
        )
        .unwrap();
    let mut second = first.clone();
    second.version = first.version.next().unwrap();
    second.name = "later-version.txt".into();
    second.vault.push(9);
    store
        .append(
            replacement,
            f.case,
            first.version,
            second,
            at + Duration::days(1),
        )
        .unwrap();
    store
        .replace_metadata(
            replacement,
            f.case,
            first.id,
            MetadataRevision::unclassified(),
            metadata("Resolution", "Private", &[]),
            at + Duration::days(2),
        )
        .unwrap();
    drop(store);
    let _reconnected = f.store();
    assert_eq!(checked_origin(&mut f, &first, uploader, at), saved);
    assert_eq!(
        f.db.client
            .query_one("SELECT count(*) FROM document_upload_origins", &[])
            .unwrap()
            .get::<_, i64>(0),
        1
    );
}

#[test]
fn classified_upload_has_one_origin_linked_to_its_upload_event() {
    let Some(mut f) = Fixture::new() else { return };
    let store = f.store();
    let record = document();
    let at = f.at;
    let owner = f.owner;
    store
        .insert_with_metadata(
            owner,
            f.case,
            record.clone(),
            metadata("Resolution", "Private", &[]),
            at,
        )
        .unwrap();
    checked_origin(&mut f, &record, owner, at);
    let before = snapshot(&mut f);
    assert!(matches!(
        store.insert(owner, f.case, record, at + Duration::days(1)),
        Err(ApplicationError::DocumentAlreadyExists(_))
    ));
    assert!(f
        .db
        .client
        .execute(
            "INSERT INTO document_upload_origins SELECT * FROM document_upload_origins",
            &[]
        )
        .is_err());
    assert_eq!(snapshot(&mut f), before);
}

#[test]
fn failed_origin_audit_or_commit_rolls_back_the_entire_classified_upload() {
    for failure in ["origin", "audit", "commit"] {
        let Some(mut f) = Fixture::new() else { return };
        let store = f.store();
        if failure == "audit" {
            f.db.client
                .batch_execute(
                    "ALTER TABLE audit_events ADD CONSTRAINT reject_upload_audit
                     CHECK(action<>'document.uploaded')",
                )
                .unwrap();
        } else {
            let (constraint, timing, deferred) = if failure == "commit" {
                ("CONSTRAINT ", "AFTER", "DEFERRABLE INITIALLY DEFERRED")
            } else {
                ("", "BEFORE", "")
            };
            f.db.client
                .batch_execute(&format!(
                    "CREATE FUNCTION reject_upload_origin() RETURNS TRIGGER LANGUAGE plpgsql
                     AS $$ BEGIN RAISE EXCEPTION 'injected upload origin failure'; END $$;
                     CREATE {constraint}TRIGGER reject_upload_origin {timing} INSERT
                     ON document_upload_origins {deferred} FOR EACH ROW
                     EXECUTE FUNCTION reject_upload_origin()"
                ))
                .unwrap();
        }
        let before = snapshot(&mut f);
        let result = store.insert_with_metadata(
            f.owner,
            f.case,
            document(),
            metadata("Resolution", "Private", &[]),
            f.at,
        );
        assert!(
            matches!(result, Err(ApplicationError::Port(_))),
            "failure={failure}"
        );
        assert_eq!(snapshot(&mut f), before, "failure={failure}");
    }
}

#[test]
fn migration_and_later_append_do_not_infer_a_legacy_uploader_from_email() {
    let Some(mut db) = Database::old_schema() else {
        return;
    };
    let case = CaseId::from_uuid(db.seed_case());
    let owner = UserId::from_uuid(
        db.client
            .query_one(
                "SELECT created_by FROM cases WHERE id=$1",
                &[&case.as_uuid()],
            )
            .unwrap()
            .get(0),
    );
    let first = document();
    db.client
        .execute(
            "INSERT INTO documents(id,case_id,version,name,digest,vault)
             VALUES($1,$2,1,$3,$4,$5)",
            &[
                &first.id.as_uuid(),
                &case.as_uuid(),
                &first.name,
                &&first.digest.as_bytes()[..],
                &first.vault,
            ],
        )
        .unwrap();
    let at = OffsetDateTime::from_unix_timestamp_nanos(1_735_689_600_123_456_789).unwrap();
    let event = AuditEvent::new(
        0,
        at,
        format!("{owner}@example.test"),
        "document.uploaded",
        format!(
            "case:{case}:document:{}:version:1:sha256:{}",
            first.id,
            first.digest.to_hex()
        ),
    );
    let chain = chain_digest(&RingSha256Hasher::new(), &GENESIS_PREVIOUS, &event).unwrap();
    db.client
        .execute(
            "INSERT INTO audit_events(sequence,timestamp,actor,action,resource,chain)
             VALUES(0,$1,$2,$3,$4,$5)",
            &[
                &event.timestamp_rfc3339().unwrap(),
                &event.actor,
                &event.action,
                &event.resource,
                &&chain.as_bytes()[..],
            ],
        )
        .unwrap();
    let before = db.snapshot();
    for _ in 0..2 {
        PostgresCaseDocumentStore::connect(&db.url).unwrap();
        assert_eq!(db.snapshot(), before);
        assert_no_legacy_origin(&mut db, first.id);
    }
    let store = PostgresCaseDocumentStore::connect(&db.url).unwrap();
    let mut second = first.clone();
    second.version = first.version.next().unwrap();
    second.vault.push(9);
    store
        .append(owner, case, first.version, second, at + Duration::days(1))
        .unwrap();
    assert_no_legacy_origin(&mut db, first.id);
}

fn assert_no_legacy_origin(db: &mut Database, id: DocumentId) {
    assert_eq!(
        db.client
            .query_one(
                "SELECT count(*) FROM document_upload_origins WHERE document_id=$1",
                &[&id.as_uuid()],
            )
            .unwrap()
            .get::<_, i64>(0),
        0
    );
}
