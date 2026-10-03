use postgres::{Client, NoTls};
use uuid::Uuid;

use crate::support::*;

struct Grant {
    client: Client,
    runtime: String,
    delegated: Option<String>,
}

impl Grant {
    fn new(db: &Fixture, delegated: bool) -> Self {
        let mut grant = Self {
            client: Client::connect(&db.admin_url, NoTls).unwrap(),
            runtime: db.role.clone(),
            delegated: delegated.then(|| format!("owner_replication_{}", Uuid::new_v4().simple())),
        };
        if let Some(role) = &grant.delegated {
            grant
                .client
                .batch_execute(&format!(
                    "CREATE ROLE {role} NOLOGIN;
                ALTER ROLE {} NOINHERIT; GRANT {role} TO {};
                GRANT SET ON PARAMETER session_replication_role TO {role}",
                    grant.runtime, grant.runtime
                ))
                .unwrap();
        } else {
            grant
                .client
                .batch_execute(&format!(
                    "GRANT SET ON PARAMETER session_replication_role TO {}",
                    grant.runtime
                ))
                .unwrap();
        }
        grant
    }

    fn demonstrate_capability(&mut self, db: &Fixture) {
        let mut runtime = db.runtime();
        if let Some(role) = &self.delegated {
            let reachable: bool = runtime
                .query_one("SELECT pg_has_role(current_user,$1,'MEMBER')", &[role])
                .unwrap()
                .get(0);
            assert!(reachable);
            runtime.batch_execute(&format!("SET ROLE {role}")).unwrap();
        }
        let permitted: bool = runtime
            .query_one(
                "SELECT has_parameter_privilege(current_user,'session_replication_role','SET')",
                &[],
            )
            .unwrap()
            .get(0);
        assert!(
            permitted,
            "fixture did not grant the trigger-bypass capability"
        );
        runtime.batch_execute("RESET ROLE").unwrap();
        let mode: String = self
            .client
            .query_one("SHOW session_replication_role", &[])
            .unwrap()
            .get(0);
        assert_eq!(mode, "origin");
    }
}

impl Drop for Grant {
    fn drop(&mut self) {
        if let Some(role) = &self.delegated {
            let _ = self.client.batch_execute(&format!(
                "REVOKE SET ON PARAMETER session_replication_role FROM {role};
                REVOKE {role} FROM {}; ALTER ROLE {} INHERIT; DROP ROLE {role}",
                self.runtime, self.runtime
            ));
        } else {
            let _ = self.client.batch_execute(&format!(
                "REVOKE SET ON PARAMETER session_replication_role FROM {}",
                self.runtime
            ));
        }
    }
}

fn denied(delegated: bool) {
    let Some((mut db, clock, _)) = fixture() else {
        return;
    };
    drop(store(&db, &clock));
    let before = snapshot(&mut db);
    let mut grant = Grant::new(&db, delegated);
    grant.demonstrate_capability(&db);
    let rejected = PostgresOwnerCertificateStore::open(&db.runtime_url, clock).is_err();
    assert_eq!(snapshot(&mut db), before);
    drop(grant);
    assert!(
        rejected,
        "runtime with trigger-bypass authority must fail startup validation"
    );
}

#[test]
fn runtime_cannot_hold_replication_mode_authority_directly() {
    denied(false);
}

#[test]
fn runtime_cannot_reach_replication_mode_authority_through_set_role() {
    denied(true);
}
