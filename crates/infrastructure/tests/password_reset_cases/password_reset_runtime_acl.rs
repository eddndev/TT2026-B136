use super::password_reset_runtime_support::*;
use redis::Commands;
use std::time::Duration;

struct RestrictedUser {
    admin: redis::Connection,
    name: String,
    url: String,
}

impl RestrictedUser {
    fn create(db: &Fixture, global: &str, subject: &str, writable: &[&str], expiry: bool) -> Self {
        let mut admin = redis::Client::open(db.url.as_str())
            .unwrap()
            .get_connection()
            .unwrap();
        let name = format!("reset-budget-acl-{}", uuid::Uuid::new_v4().simple());
        let existing: Option<redis::Value> = redis::cmd("ACL")
            .arg("GETUSER")
            .arg(&name)
            .query(&mut admin)
            .unwrap();
        assert!(
            existing.is_none(),
            "ACL fixture refuses to replace an existing user"
        );
        let password = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let mut url = redis::parse_redis_url(&db.url).expect("fixture Redis URL is invalid");
        url.set_username(&name)
            .expect("fixture Redis URL cannot accept a username");
        url.set_password(Some(&password))
            .expect("fixture Redis URL cannot accept a password");
        let mut user = Self {
            admin,
            name,
            url: url.into(),
        };
        let read = format!(
            concat!(
                "(-@all +auth +select +client|setinfo +eval +evalsha +script|load ",
                "+time +type +hlen +hmget +pttl +pexpiretime ~{} ~{})"
            ),
            global, subject
        );
        let mut write = String::from("(-@all +hset");
        if expiry {
            write.push_str(" +pexpireat");
        }
        for key in writable {
            write.push_str(" ~");
            write.push_str(key);
        }
        write.push(')');
        redis::cmd("ACL")
            .arg("SETUSER")
            .arg(&user.name)
            .arg("reset")
            .arg("on")
            .arg(format!(">{password}"))
            .arg(read)
            .arg(write)
            .query::<()>(&mut user.admin)
            .unwrap_or_else(|_| panic!("restricted ACL fixture could not be provisioned"));
        let entered = user.permits("EVAL", &["return redis.call('TIME')", "2", global, subject]);
        assert!(
            entered,
            "ACL fixture blocks script invocation before its internal checks"
        );
        user
    }

    fn permits(&mut self, command: &str, args: &[&str]) -> bool {
        redis::cmd("ACL")
            .arg("DRYRUN")
            .arg(&self.name)
            .arg(command)
            .arg(args)
            .query::<String>(&mut self.admin)
            .map(|reply| reply == "OK")
            .unwrap_or(false)
    }

    fn store(&self) -> RedisPasswordResetLimiter {
        RedisPasswordResetLimiter::connect(
            &self.url,
            policy(3, 2),
            Duration::from_secs(1),
            Duration::from_secs(1),
        )
        .unwrap()
    }
}

impl Drop for RestrictedUser {
    fn drop(&mut self) {
        let _ = redis::cmd("ACL")
            .arg("DELUSER")
            .arg(&self.name)
            .query::<u64>(&mut self.admin);
    }
}

fn reject_one_writable_budget(existing: bool) {
    for completion in [false, true] {
        for writable_global in [false, true] {
            let mut db = Fixture::new();
            let (email, email_key) = db.email();
            let (digest, digest_key) = db.completion(53);
            let (global, subject) = if completion {
                (COMPLETION_GLOBAL, digest_key.as_str())
            } else {
                (REQUEST_GLOBAL, email_key.as_str())
            };
            if existing {
                let admin = db.store(policy(3, 2));
                assert!(if completion {
                    admin.admit_completion(digest)
                } else {
                    admin.admit_request(&email)
                }
                .unwrap());
            }
            let writable = if writable_global { global } else { subject };
            let denied = if writable_global { subject } else { global };
            let mut user = RestrictedUser::create(&db, global, subject, &[writable], true);
            assert!(user.permits("HSET", &[writable, "count", "2"]));
            assert!(!user.permits("HSET", &[denied, "count", "2"]));
            assert!(user.permits("PEXPIREAT", &[writable, "9999999999999"]));
            assert!(!user.permits("PEXPIREAT", &[denied, "9999999999999"]));
            let before = db.snapshot();
            let store = user.store();
            let result = if completion {
                store.admit_completion(digest)
            } else {
                store.admit_request(&email)
            };
            let error = result.expect_err("restricted write must fail closed, not return quota");
            assert!(
                db.snapshot() == before,
                "ACL rejection partially charged or created a budget"
            );
            assert!(!error.to_string().contains(&email) && !error.to_string().contains(&user.name));
            if !existing {
                assert!(!db.connection.exists::<_, bool>(global).unwrap());
                assert!(!db.connection.exists::<_, bool>(subject).unwrap());
            }
            let name = user.name.clone();
            drop(user);
            let removed: Option<redis::Value> = redis::cmd("ACL")
                .arg("GETUSER")
                .arg(name)
                .query(&mut db.connection)
                .unwrap();
            assert!(removed.is_none(), "owned restricted user was not removed");
        }
    }
}

#[test]
fn restricted_write_to_one_budget_cannot_partially_create_either_counter() {
    reject_one_writable_budget(false);
}

#[test]
fn restricted_write_to_one_budget_cannot_partially_charge_existing_counters() {
    reject_one_writable_budget(true);
}

#[test]
fn denied_expiry_permission_cannot_leave_a_new_counter_without_expiration() {
    let mut db = Fixture::new();
    let (email, subject) = db.email();
    let mut user = RestrictedUser::create(
        &db,
        REQUEST_GLOBAL,
        &subject,
        &[REQUEST_GLOBAL, &subject],
        false,
    );
    for key in [REQUEST_GLOBAL, subject.as_str()] {
        assert!(user.permits("HSET", &[key, "count", "1"]));
        assert!(!user.permits("PEXPIREAT", &[key, "9999999999999"]));
    }
    let before = db.snapshot();
    assert!(user.store().admit_request(&email).is_err());
    assert!(
        db.snapshot() == before,
        "expiry ACL rejection left a partial counter"
    );
}
