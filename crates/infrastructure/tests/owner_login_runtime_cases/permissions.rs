use super::{material, support::*};

struct RestrictedUser {
    admin: redis::Connection,
    name: String,
    url: String,
}

impl RestrictedUser {
    fn new(
        db: &Fixture,
        global: &str,
        subject: &str,
        writable: &[&str],
        creation: bool,
        expiry: bool,
        consume: bool,
    ) -> Self {
        let mut admin = redis::Client::open(db.url.as_str())
            .unwrap()
            .get_connection()
            .unwrap();
        let name = format!("owner-login-acl-{}", Uuid::new_v4().simple());
        let existing: Option<redis::Value> = redis::cmd("ACL")
            .arg("GETUSER")
            .arg(&name)
            .query(&mut admin)
            .unwrap();
        assert!(
            existing.is_none(),
            "fixture refuses to replace existing ACL user"
        );
        let password = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        let mut url = redis::parse_redis_url(&db.url).unwrap();
        url.set_username(&name).unwrap();
        url.set_password(Some(&password)).unwrap();
        let mut user = Self {
            admin,
            name,
            url: url.into(),
        };
        let read = format!(concat!("(-@all +auth +select +client|setinfo +eval +evalsha +script|load ",
            "+time +type +get +strlen +hlen +hmget +pttl +pexpiretime ~identity:certificate-login:* ~{} ~{})"), global, subject);
        let mut write = String::from("(-@all +hset ~identity:certificate-login:*");
        if creation {
            write.push_str(" +set");
        }
        if expiry {
            write.push_str(" +pexpireat");
        }
        if consume {
            write.push_str(" +del +getdel");
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
            .unwrap_or_else(|_| panic!("could not provision isolated ACL fixture"));
        assert!(
            user.permits("EVAL", &["return redis.call('TIME')", "2", global, subject]),
            "fixture must admit script before internal checks"
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

    fn store(&self) -> RedisOwnerLoginRuntime {
        connect(&self.url, policy(3, 2))
    }

    fn opaque(&self, result: Result<(), ApplicationError>) {
        let error = result.expect_err("ACL rejection must fail closed");
        assert!(
            matches!(error, ApplicationError::Port(_)),
            "ACL error became quota or absence"
        );
        assert!(!error.to_string().contains(&self.name));
        assert!(!error.to_string().contains(&self.url));
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

#[test]
fn denied_creation_and_budget_expiry_permissions_leave_state_intact() {
    let mut db = Fixture::new();
    let (owner, binding, subject) = db.subject();
    let (token, capture_key) = db.token();
    let proof = proof_key(&token);
    let value = material::capture(db.now() / 1000, 90);
    let mut user = RestrictedUser::new(
        &db,
        START_GLOBAL,
        &subject,
        &[START_GLOBAL, subject.as_str()],
        false,
        true,
        true,
    );
    assert!(!user.permits("SET", &[&capture_key, "public-value"]));
    assert!(user.permits("PEXPIREAT", &[&capture_key, "9999999999999"]));
    let keys = db.keys();
    let before = db.snapshot();
    user.opaque(user.store().create(&value, 90).map(|_| ()));
    assert_eq!(
        db.keys(),
        keys,
        "failed capture write left an untracked key"
    );
    drop(user);
    let mut user = RestrictedUser::new(
        &db,
        START_GLOBAL,
        &subject,
        &[START_GLOBAL, subject.as_str()],
        true,
        false,
        true,
    );
    assert!(user.permits("HSET", &[START_GLOBAL, "count", "1"]));
    assert!(!user.permits("PEXPIREAT", &[START_GLOBAL, "9999999999999"]));
    user.opaque(user.store().admit_start(owner, binding));
    assert_eq!(
        db.snapshot(),
        before,
        "failed budget expiry partially wrote state"
    );
    drop(user);
    let user = RestrictedUser::new(
        &db,
        PROOF_GLOBAL,
        &proof,
        &[PROOF_GLOBAL, proof.as_str()],
        true,
        false,
        true,
    );
    user.opaque(user.store().admit_proof(&token));
    assert_eq!(db.snapshot(), before);
}

#[test]
fn missing_one_budget_write_permission_cannot_charge_either_existing_or_absent_counter() {
    for proof in [false, true] {
        for existing in [false, true] {
            for write_global in [false, true] {
                let mut db = Fixture::new();
                let (owner, binding, start) = db.subject();
                let (token, _) = db.token();
                let proof_subject = proof_key(&token);
                let (global, subject) = if proof {
                    (PROOF_GLOBAL, proof_subject.as_str())
                } else {
                    (START_GLOBAL, start.as_str())
                };
                let admit = |store: &RedisOwnerLoginRuntime| {
                    if proof {
                        store.admit_proof(&token)
                    } else {
                        store.admit_start(owner, binding)
                    }
                };
                if existing {
                    admit(&db.store(policy(3, 2))).unwrap();
                }
                let (writable, denied) = if write_global {
                    (global, subject)
                } else {
                    (subject, global)
                };
                let mut user =
                    RestrictedUser::new(&db, global, subject, &[writable], true, true, true);
                assert!(user.permits("HSET", &[writable, "count", "2"]));
                assert!(!user.permits("HSET", &[denied, "count", "2"]));
                let before = db.snapshot();
                user.opaque(admit(&user.store()));
                assert_eq!(
                    db.snapshot(),
                    before,
                    "one writable key caused a partial charge"
                );
            }
        }
    }
}

#[test]
fn failed_consume_permission_is_an_error_and_cannot_return_a_replayable_capture() {
    let mut db = Fixture::new();
    let value = material::capture(db.now() / 1000, 90);
    let (token, key) = db.issue(&value, 90);
    let (_, _, subject) = db.subject();
    let mut user = RestrictedUser::new(&db, START_GLOBAL, &subject, &[], true, true, false);
    assert!(user.permits("GET", &[&key]));
    assert!(!user.permits("GETDEL", &[&key]));
    assert!(!user.permits("DEL", &[&key]));
    let before = db.read(&key);
    user.opaque(user.store().take(&token).map(|_| ()));
    assert_eq!(
        db.read(&key),
        before,
        "failed consumption changed stored proof"
    );
    assert_eq!(db.store(policy(3, 2)).take(&token).unwrap(), Some(value));
}
