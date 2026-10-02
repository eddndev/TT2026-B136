use crate::case_administration_support::Fixture;
use application::ApplicationError;
use postgres::Client;

pub(super) fn bounded_url(db: &Fixture) -> (String, String) {
    let application = format!("startup_scope_{}", uuid::Uuid::new_v4().simple());
    let mut url = reqwest::Url::parse(&db.runtime_url).unwrap();
    let values: Vec<_> = url
        .query_pairs()
        .map(|(key, value)| {
            if key == "options" {
                (key.into_owned(), format!("{value} -clock_timeout=250ms"))
            } else {
                (key.into_owned(), value.into_owned())
            }
        })
        .collect();
    url.query_pairs_mut()
        .clear()
        .extend_pairs(values)
        .append_pair("application_name", &application);
    // PostgreSQL URI options decode percent escapes, not form-encoded spaces.
    (url.to_string().replace('+', "%20"), application)
}

pub(super) fn mutation_lock_available(client: &mut Client) -> bool {
    // Same canonical mutation key used by the versioned audit boundary.
    let acquired: bool = client
        .query_one("SELECT pg_try_advisory_lock(280603412820)", &[])
        .unwrap()
        .get(0);
    if acquired {
        client
            .query_one("SELECT pg_advisory_unlock(280603412820)", &[])
            .unwrap();
    }
    acquired
}

pub(super) fn backend_pids(client: &mut Client, application: &str) -> Vec<i32> {
    client
        .query(
            "SELECT pid FROM pg_stat_activity WHERE application_name=$1 ORDER BY pid",
            &[&application],
        )
        .unwrap()
        .into_iter()
        .map(|row| row.get(0))
        .collect()
}

pub(super) fn corrupt_case_actor_email(db: &mut Fixture) {
    db.admin.execute("INSERT INTO case_administration_revisions(
        case_id,revision,title,reference,administrative_status,values_digest,
        changed_at,changed_by,changed_by_email)
        VALUES($1,1,'Baseline','REF-OLD','active',
        sha256(case_administration_bytes('active','Baseline','REF-OLD',NULL,NULL,NULL,NULL,NULL,NULL,NULL)),
        '2025-01-01T00:00:00Z',$2,' invalid ')", &[&db.case.as_uuid(), &db.owner.as_uuid()]).unwrap();
}

pub(super) fn assert_inventory_error(error: ApplicationError) {
    assert!(
        matches!(error, ApplicationError::InvalidConfiguration(ref message)
        if message.contains("case administration inventory is inconsistent")),
        "{error}"
    );
}
