use super::port;
use application::{
    members::{validate_user_summary, MemberError, UserSummary},
    ApplicationError,
};
use domain::identity::{Role, UserId};
use postgres::GenericClient;
use std::str::FromStr;

pub(crate) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let invalid: bool = client.query_one("SELECT
        (EXISTS(SELECT 1 FROM users) AND NOT EXISTS(SELECT 1 FROM users WHERE active AND role='owner'))
        OR EXISTS(SELECT 1 FROM users WHERE auth_generation<0 OR revision<0 OR auth_generation>revision)
        OR EXISTS(SELECT 1 FROM case_memberships m LEFT JOIN users u ON u.id=m.user_id
            LEFT JOIN cases c ON c.id=m.case_id WHERE u.id IS NULL OR c.id IS NULL
                OR NOT isfinite(m.assigned_at) OR m.assigned_at<'0001-01-01Z'::timestamptz
                OR m.assigned_at>='10000-01-01Z'::timestamptz)", &[]).map_err(port)?.get(0);
    if invalid {
        return Err(MemberError::Stored("invalid account or membership inventory").into());
    }
    let mut after: Option<uuid::Uuid> = None;
    loop {
        let rows = client
            .query(
                "SELECT id,email,role,active,revision FROM users
            WHERE ($1::uuid IS NULL OR id>$1) ORDER BY id LIMIT 64",
                &[&after],
            )
            .map_err(port)?;
        if rows.is_empty() {
            break;
        }
        for row in rows {
            let user = UserSummary {
                id: UserId::from_uuid(row.get("id")),
                email: row.get("email"),
                role: Role::from_str(row.get::<_, &str>("role"))
                    .map_err(|_| MemberError::Stored("invalid user role"))?,
                active: row.get("active"),
                revision: u64::try_from(row.get::<_, i64>("revision"))
                    .map_err(|_| MemberError::Stored("invalid user revision"))?,
            };
            validate_user_summary(&user)?;
            after = Some(user.id.as_uuid());
        }
    }
    Ok(())
}
