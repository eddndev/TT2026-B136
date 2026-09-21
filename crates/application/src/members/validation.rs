use super::{
    CaseMemberPage, CaseMemberQuery, MemberError, MemberSelection, OffsetDateTime, Role,
    UserAccessChange, UserId, UserPage, UserQuery, UserStatusFilter, UserSummary,
};

/// Validates a public user projection without reading or returning credentials.
pub fn validate_user_summary(user: &UserSummary) -> Result<(), MemberError> {
    if user.email.len() > 254 || !user.email.is_ascii() {
        return Err(MemberError::Stored("user email is invalid"));
    }
    let email = crate::identity::normalize_email(&user.email)
        .map_err(|_| MemberError::Stored("user email is invalid"))?;
    if email != user.email || user.revision > i64::MAX as u64 {
        return Err(MemberError::Stored(
            "user email or revision is not canonical",
        ));
    }
    Ok(())
}

pub(super) fn window(start: OffsetDateTime, end: OffsetDateTime) -> Result<(), MemberError> {
    if end < start || !(1..=9999).contains(&start.year()) || !(1..=9999).contains(&end.year()) {
        return Err(MemberError::Stored("directory observation time is invalid"));
    }
    Ok(())
}

fn matches_filters(user: &UserSummary, role: Option<Role>, prefix: Option<&str>) -> bool {
    role.is_none_or(|role| role == user.role)
        && prefix.is_none_or(|prefix| user.email.starts_with(prefix))
}

fn ordered(previous: Option<UserId>, current: UserId) -> Result<(), MemberError> {
    if previous.is_some_and(|previous| previous.as_uuid() >= current.as_uuid()) {
        return Err(MemberError::Stored(
            "directory identifiers are not increasing",
        ));
    }
    Ok(())
}

fn continuation(
    count: usize,
    limit: u32,
    has_more: bool,
    cursor: Option<&str>,
    expected: Option<String>,
) -> Result<(), MemberError> {
    if count > limit as usize {
        return Err(MemberError::Stored("directory page exceeds its limit"));
    }
    let valid = if has_more {
        count == limit as usize && expected.is_some() && cursor == expected.as_deref()
    } else {
        cursor.is_none()
    };
    if !valid {
        return Err(MemberError::Stored(
            "directory continuation is inconsistent",
        ));
    }
    Ok(())
}

pub(super) fn users(page: &UserPage, query: &UserQuery) -> Result<(), MemberError> {
    if page.items.len() > query.limit() as usize {
        return Err(MemberError::Stored("directory page exceeds its limit"));
    }
    let mut previous = query.after_id();
    for user in &page.items {
        validate_user_summary(user)?;
        ordered(previous, user.id)?;
        let status_matches = match query.status() {
            UserStatusFilter::Active => user.active,
            UserStatusFilter::Inactive => !user.active,
            UserStatusFilter::All => true,
        };
        if !status_matches || !matches_filters(user, query.role(), query.email_prefix()) {
            return Err(MemberError::Stored(
                "directory item differs from its filters",
            ));
        }
        previous = Some(user.id);
    }
    continuation(
        page.items.len(),
        query.limit(),
        page.has_more,
        page.next_cursor.as_deref(),
        page.items.last().map(|user| query.cursor_after(user.id)),
    )
}

pub(super) fn case_members(
    page: &CaseMemberPage,
    query: &CaseMemberQuery,
    returned_at: OffsetDateTime,
) -> Result<(), MemberError> {
    if page.items.len() > query.limit() as usize {
        return Err(MemberError::Stored("member page exceeds its limit"));
    }
    if page.case_id != query.case_id() {
        return Err(MemberError::Stored(
            "member page belongs to a different case",
        ));
    }
    let mut previous = query.after_id();
    for item in &page.items {
        validate_user_summary(&item.user)?;
        ordered(previous, item.user.id)?;
        if !matches_filters(&item.user, query.role(), query.email_prefix()) {
            return Err(MemberError::Stored("member differs from its filters"));
        }
        match (query.selection(), item.assigned_at) {
            (MemberSelection::Assigned, Some(at))
                if at <= returned_at
                    && (1..=9999).contains(&at.year())
                    && at.offset() == time::UtcOffset::UTC => {}
            (MemberSelection::Available, None) if item.user.active => {}
            _ => {
                return Err(MemberError::Stored(
                    "member assignment selection is inconsistent",
                ))
            }
        }
        previous = Some(item.user.id);
    }
    continuation(
        page.items.len(),
        query.limit(),
        page.has_more,
        page.next_cursor.as_deref(),
        page.items
            .last()
            .map(|item| query.cursor_after(item.user.id)),
    )
}

pub(super) fn access(
    user: &UserSummary,
    id: UserId,
    change: UserAccessChange,
) -> Result<(), MemberError> {
    validate_user_summary(user)?;
    let same_revision = user.revision == change.expected_revision();
    let next_revision = change.expected_revision().checked_add(1) == Some(user.revision);
    if user.id != id
        || user.role != change.role()
        || user.active != change.active()
        || !(same_revision || next_revision)
    {
        return Err(MemberError::Stored(
            "confirmed access differs from its command",
        ));
    }
    Ok(())
}
