//! A direct client and a `PreparedCatalog` must reach the same verdict on real
//! migrated catalogs, and a complete runtime validation must leave no prepared
//! statement on its connection. See
//! docs/adr/0083-call-scoped-prepared-catalog-statements.md.

use super::tests::prepared_statements;
use super::{CatalogClient, PreparedCatalog};
use crate::postgres::test_fixture::CatalogFixture;
use crate::postgres::{connect_runtime, validate_catalog, validate_runtime};

/// Catalog tampering that the alert catalog or runtime role checks reject.
/// The renamed table makes a role check fail with a database error.
const MUTATIONS: &[&str] = &[
    "ALTER TABLE alert_read_receipts RENAME TO alert_read_receipts_moved",
    "ALTER TABLE alert_schedule DISABLE TRIGGER alert_lock",
    "ALTER FUNCTION alert_payload_valid(bytea,bytea) SECURITY DEFINER",
    "ALTER TABLE alert_email_attempts DROP CONSTRAINT alert_email_attempts_sequence_check;
    ALTER TABLE alert_email_attempts ADD CONSTRAINT alert_email_attempts_sequence_check CHECK(true)",
    "DROP INDEX alert_inbox_order;
    CREATE INDEX alert_inbox_order ON alert_notifications(recipient,created_seconds DESC,created_nanos,id)",
    "GRANT DELETE ON alert_schedule TO {runtime}",
    "CREATE ROLE {helper} NOLOGIN; GRANT TRUNCATE ON alert_notifications TO {helper};
    ALTER ROLE {runtime} NOINHERIT; GRANT {helper} TO {runtime}",
];

#[derive(Clone, Copy, Debug)]
enum Check {
    AlertCatalog,
    AlertRuntimeRole,
    CatalogAndRuntimeRole,
}

const CHECKS: [Check; 3] = [
    Check::AlertCatalog,
    Check::AlertRuntimeRole,
    Check::CatalogAndRuntimeRole,
];

fn verdict<C: CatalogClient>(client: &mut C, check: Check, role: &str) -> String {
    let result = match check {
        Check::AlertCatalog => crate::alert_schema::validate(client),
        Check::AlertRuntimeRole => crate::alert_schema::validate_runtime_role(client, role),
        Check::CatalogAndRuntimeRole => validate_catalog(client),
    };
    format!("{result:?}")
}

#[test]
fn direct_and_prepared_clients_reach_the_same_verdict_on_mutated_catalogs() {
    let Some(mut db) = CatalogFixture::new() else {
        return;
    };
    let role = db.role.clone();
    for mutation in std::iter::once("").chain(MUTATIONS.iter().copied()) {
        let mut state = db.mutated(mutation).unwrap();
        let mut verdicts = Vec::new();
        for check in CHECKS {
            let direct = state.run(|client| verdict(client, check, &role));
            let prepared =
                state.run(|client| verdict(&mut PreparedCatalog::new(client), check, &role));
            assert_eq!(direct, prepared, "{check:?} after {mutation}");
            verdicts.push(direct);
        }
        if mutation.is_empty() {
            assert_eq!(verdicts, ["Ok(())"; 3], "unaltered catalog");
        } else {
            assert_ne!(verdicts[2], "Ok(())", "accepted {mutation}");
            assert!(
                verdicts[..2].iter().any(|verdict| verdict != "Ok(())"),
                "alert checks accepted {mutation}"
            );
        }
    }
}

#[test]
fn runtime_validation_leaves_no_prepared_statement_on_its_connection() {
    let Some(db) = CatalogFixture::new() else {
        return;
    };
    let mut client = connect_runtime(&db.runtime_url).unwrap();
    for _ in 0..2 {
        validate_runtime(&mut client).unwrap();
        assert_eq!(prepared_statements(&mut client), "0");
    }
}
