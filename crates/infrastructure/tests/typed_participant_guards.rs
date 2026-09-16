mod case_administration_support;
mod typed_participant_database_support;
use case_administration_support::Fixture;
use typed_participant_database_support::Bundle;

fn populated() -> Option<(Fixture, Bundle)> {
    let mut db = Fixture::new()?;
    let mut bundle = Bundle::new(&db);
    let mut client = postgres::Client::connect(&db.admin_url, postgres::NoTls).unwrap();
    let mut tx = client.transaction().unwrap();
    bundle.seed_document(&mut tx, &db);
    bundle.seed_manual(&mut tx, &db);
    bundle.refresh_stamp(&db);
    bundle.insert(&mut tx, &db);
    tx.commit().unwrap();
    assert_eq!(
        db.admin
            .query_one("SELECT count(*) FROM case_participant_revisions", &[])
            .unwrap()
            .get::<_, i64>(0),
        1
    );
    Some((db, bundle))
}
#[test]
fn typed_rows_are_immutable_even_when_update_repeats_the_same_values() {
    let Some((mut db, _)) = populated() else {
        return;
    };
    for table in [
        "case_subjects",
        "case_subject_revisions",
        "case_participant_typed_revisions",
        "subject_identity_reviews",
        "participant_identity_reviews",
    ] {
        let field = if table == "case_subjects" {
            "id"
        } else {
            "revision"
        };
        let error = db
            .admin
            .batch_execute(&format!("UPDATE {table} SET {field}={field}"))
            .unwrap_err();
        assert_eq!(
            error.code(),
            Some(&postgres::error::SqlState::CHECK_VIOLATION),
            "{table}"
        );
        let error = db
            .admin
            .batch_execute(&format!("DELETE FROM {table}"))
            .unwrap_err();
        assert_eq!(
            error.code(),
            Some(&postgres::error::SqlState::CHECK_VIOLATION),
            "{table}"
        );
    }
}
#[test]
fn manual_route_cannot_append_after_explicit_typing() {
    let Some((mut db, bundle)) = populated() else {
        return;
    };
    let error=db.admin.execute("INSERT INTO case_participant_revisions SELECT participant_id,2,display_name,procedural_role,organization,legal_status,directory_status,values_digest,changed_at,changed_by,changed_by_email FROM case_participant_revisions WHERE participant_id=$1",&[&bundle.participant.as_uuid()]).unwrap_err();
    assert_eq!(
        error.code(),
        Some(&postgres::error::SqlState::CHECK_VIOLATION)
    );
    db.migrate();
}
