use super::{
    port,
    values::{owner, summary},
    PostgresMemberStore,
};
use crate::audit_postgres::{append_transaction, begin_audited};
use application::{
    members::{
        CaseMemberItem, CaseMemberPage, CaseMemberQuery, MemberError, UserPage, UserQuery,
        UserSummary,
    },
    ApplicationError,
};
use domain::{cases::CaseId, identity::UserId};
use time::OffsetDateTime;

impl PostgresMemberStore {
    pub(super) fn list_users(
        &self,
        actor: UserId,
        query: UserQuery,
        lower: OffsetDateTime,
    ) -> Result<UserPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let email = owner(&mut tx, actor)?;
        let at = self.checked_at(lower)?;
        let after = query.after_id().map(UserId::as_uuid);
        let role = query.role().map(|role| role.as_str());
        let rows = tx
            .query(
                "SELECT id,email,role,active,revision FROM users
            WHERE ($1::uuid IS NULL OR id>$1) AND ($2='all' OR active=($2='active'))
            AND ($3::text IS NULL OR role=$3)
            AND ($4::text IS NULL OR left(email,length($4))=$4)
            ORDER BY id LIMIT $5",
                &[
                    &after,
                    &query.status().as_str(),
                    &role,
                    &query.email_prefix(),
                    &(i64::from(query.limit()) + 1),
                ],
            )
            .map_err(port)?;
        let mut items = rows.iter().map(summary).collect::<Result<Vec<_>, _>>()?;
        let has_more = items.len() > query.limit() as usize;
        items.truncate(query.limit() as usize);
        let next_cursor = if has_more {
            items.last().map(|user| query.cursor_after(user.id))
        } else {
            None
        };
        append_transaction(&mut tx, &email, "identity.users_listed", "users", at)?;
        tx.commit().map_err(port)?;
        Ok(UserPage {
            items,
            has_more,
            next_cursor,
        })
    }

    pub(super) fn read_user(
        &self,
        actor: UserId,
        id: UserId,
        lower: OffsetDateTime,
    ) -> Result<UserSummary, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let email = owner(&mut tx, actor)?;
        let at = self.checked_at(lower)?;
        let row = tx
            .query_opt(
                "SELECT id,email,role,active,revision FROM users WHERE id=$1",
                &[&id.as_uuid()],
            )
            .map_err(port)?
            .ok_or(ApplicationError::UserNotFound)?;
        let user = summary(&row)?;
        append_transaction(
            &mut tx,
            &email,
            "identity.user_read",
            &format!("user:{id}"),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(user)
    }

    pub(super) fn case_members(
        &self,
        actor: UserId,
        case: CaseId,
        query: CaseMemberQuery,
        lower: OffsetDateTime,
    ) -> Result<CaseMemberPage, ApplicationError> {
        let mut client = self.client()?;
        let mut tx = begin_audited(&mut client)?;
        let email = owner(&mut tx, actor)?;
        let at = self.checked_at(lower)?;
        if query.case_id() != case {
            return Err(MemberError::Invalid("member query belongs to another case").into());
        }
        tx.query_opt("SELECT id FROM cases WHERE id=$1", &[&case.as_uuid()])
            .map_err(port)?
            .ok_or(ApplicationError::CaseNotFound)?;
        let after = query.after_id().map(UserId::as_uuid);
        let role = query.role().map(|role| role.as_str());
        let rows = tx.query("SELECT u.id,u.email,u.role,u.active,u.revision,
            (extract(epoch FROM m.assigned_at)*1000000000)::numeric(30,0)::text AS assigned_at
            FROM users u LEFT JOIN case_memberships m ON m.user_id=u.id AND m.case_id=$1
            WHERE ($2::uuid IS NULL OR u.id>$2)
            AND (($3='assigned' AND m.user_id IS NOT NULL) OR ($3='available' AND m.user_id IS NULL AND u.active))
            AND ($4::text IS NULL OR u.role=$4)
            AND ($5::text IS NULL OR left(u.email,length($5))=$5)
            ORDER BY u.id LIMIT $6", &[&case.as_uuid(), &after, &query.selection().as_str(),
                &role, &query.email_prefix(), &(i64::from(query.limit())+1)]).map_err(port)?;
        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let assigned_at = row
                .get::<_, Option<String>>("assigned_at")
                .map(|value| {
                    let nanos = value
                        .parse::<i128>()
                        .map_err(|_| MemberError::Stored("invalid membership timestamp"))?;
                    OffsetDateTime::from_unix_timestamp_nanos(nanos)
                        .map_err(|_| MemberError::Stored("invalid membership timestamp"))
                })
                .transpose()?;
            if assigned_at.is_some_and(|value| {
                value.offset() != time::UtcOffset::UTC
                    || value > at
                    || !(1..=9999).contains(&value.year())
            }) {
                return Err(MemberError::Stored("invalid membership timestamp").into());
            }
            items.push(CaseMemberItem {
                user: summary(&row)?,
                assigned_at,
            });
        }
        let has_more = items.len() > query.limit() as usize;
        items.truncate(query.limit() as usize);
        let next_cursor = if has_more {
            items.last().map(|item| query.cursor_after(item.user.id))
        } else {
            None
        };
        append_transaction(
            &mut tx,
            &email,
            "case.members_listed",
            &format!("case:{case}:members"),
            at,
        )?;
        tx.commit().map_err(port)?;
        Ok(CaseMemberPage {
            case_id: case,
            items,
            has_more,
            next_cursor,
        })
    }
}
