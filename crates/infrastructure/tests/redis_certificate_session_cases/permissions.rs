use std::time::Duration;

use super::support::*;

struct RestrictedUser {
    admin: redis::Connection,
    name: String,
    url: String,
}

impl RestrictedUser {
    fn new(db: &Fixture) -> Self {
        let mut admin = redis::Client::open(db.url.as_str())
            .unwrap()
            .get_connection_with_timeout(Duration::from_secs(2))
            .unwrap();
        admin
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        admin
            .set_write_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let name = format!("certificate-session-acl-{}", uuid::Uuid::new_v4().simple());
        let existing: Option<redis::Value> = redis::cmd("ACL")
            .arg("GETUSER")
            .arg(&name)
            .query(&mut admin)
            .unwrap();
        assert!(
            existing.is_none(),
            "fixture refuses to replace an existing Redis user"
        );
        let password = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let mut url = redis::parse_redis_url(&db.url).expect("invalid disposable Redis URL");
        url.set_username(&name)
            .expect("fixture username is not supported");
        url.set_password(Some(&password))
            .expect("fixture password is not supported");
        let mut user = Self {
            admin,
            name,
            url: url.into(),
        };
        redis::cmd("ACL")
            .arg("SETUSER")
            .arg(&user.name)
            .arg("reset")
            .arg("on")
            .arg(format!(">{password}"))
            .arg("~identity:session:*")
            .arg("~identity:challenge:*")
            .arg("+auth")
            .arg("+select")
            .arg("+client|setinfo")
            .arg("+eval")
            .arg("+evalsha")
            .arg("+script|load")
            .arg("+time")
            .arg("+type")
            .arg("+exists")
            .arg("+hlen")
            .arg("+hmget")
            .arg("+hget")
            .arg("+hgetall")
            .arg("+pttl")
            .arg("+pexpiretime")
            .arg("+hset")
            .arg("+get")
            .arg("+getdel")
            .arg("+set")
            .arg("+del")
            .query::<()>(&mut user.admin)
            .unwrap_or_else(|_| panic!("restricted Redis user provisioning failed"));
        let mut connection = redis::Client::open(user.url.as_str())
            .unwrap()
            .get_connection_with_timeout(Duration::from_secs(2))
            .unwrap_or_else(|_| panic!("restricted Redis authentication failed"));
        connection
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        connection
            .set_write_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let now: (i64, i64) = redis::cmd("EVAL")
            .arg("return redis.call('TIME')")
            .arg(0)
            .query(&mut connection)
            .expect("fixture must admit the script before its writes");
        assert!(now.0 > 0);
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
fn a_denied_expiration_write_leaves_no_partial_certificate_session_or_activity() {
    let mut db = Fixture::new();
    let mut user = RestrictedUser::new(&db);
    let (_, probe) = db.reserve("session");
    assert!(user.permits("HSET", &[&probe, "field", "value"]));
    assert!(!user.permits("PEXPIREAT", &[&probe, "9999999999999"]));
    let restricted = connect(&user.url);
    let ceiling = db.now() + 90_123;
    let before = db.keys();
    let creation =
        restricted.create_certificate_session(&db.identity, &db.origin, db.policy, ceiling);
    if let Ok(grant) = &creation {
        db.track("session", &grant.access_token);
    }
    assert!(creation.is_err(), "ACL-rejected creation must fail closed");
    assert_eq!(
        db.keys(),
        before,
        "ACL rejection left a session hash without its expiration"
    );
    let (token, key, _) = db.issue(ceiling);
    db.age(&key, 10_000);
    let before = db.snapshot(&key);
    assert!(restricted
        .record_certificate_activity(&token, &db.identity, &db.origin, db.policy)
        .is_err());
    assert!(
        db.snapshot(&key) == before,
        "ACL rejection partially rewrote activity or deadline"
    );
    assert!(db.store.find_session(&token, db.policy).unwrap().is_some());
}
