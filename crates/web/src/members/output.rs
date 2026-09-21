use super::ApiError;
use application::members::{CaseMemberItem, CaseMemberPage, UserPage, UserSummary};
use serde::Serialize;
use time::format_description::well_known::Rfc3339;

#[derive(Serialize)]
pub(super) struct UserResponse {
    id: String,
    email: String,
    role: &'static str,
    active: bool,
    revision: String,
}
impl TryFrom<UserSummary> for UserResponse {
    type Error = ApiError;
    fn try_from(user: UserSummary) -> Result<Self, Self::Error> {
        if user.revision > i64::MAX as u64 {
            return Err(ApiError::internal());
        }
        Ok(Self {
            id: user.id.to_string(),
            email: user.email,
            role: user.role.as_str(),
            active: user.active,
            revision: user.revision.to_string(),
        })
    }
}
#[derive(Serialize)]
pub(super) struct DirectoryResponse {
    items: Vec<UserResponse>,
    has_more: bool,
    next_cursor: Option<String>,
}
impl TryFrom<UserPage> for DirectoryResponse {
    type Error = ApiError;
    fn try_from(page: UserPage) -> Result<Self, Self::Error> {
        Ok(Self {
            items: page
                .items
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            has_more: page.has_more,
            next_cursor: page.next_cursor,
        })
    }
}
#[derive(Serialize)]
struct AssignmentResponse {
    #[serde(flatten)]
    user: UserResponse,
    assigned_at: Option<String>,
}
impl TryFrom<CaseMemberItem> for AssignmentResponse {
    type Error = ApiError;
    fn try_from(item: CaseMemberItem) -> Result<Self, Self::Error> {
        let assigned_at = item
            .assigned_at
            .map(|at| {
                at.to_offset(time::UtcOffset::UTC)
                    .format(&Rfc3339)
                    .map_err(|_| ApiError::internal())
            })
            .transpose()?;
        Ok(Self {
            user: item.user.try_into()?,
            assigned_at,
        })
    }
}
#[derive(Serialize)]
pub(super) struct AssignmentPageResponse {
    case_id: String,
    items: Vec<AssignmentResponse>,
    has_more: bool,
    next_cursor: Option<String>,
}
impl TryFrom<CaseMemberPage> for AssignmentPageResponse {
    type Error = ApiError;
    fn try_from(page: CaseMemberPage) -> Result<Self, Self::Error> {
        Ok(Self {
            case_id: page.case_id.to_string(),
            items: page
                .items
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            has_more: page.has_more,
            next_cursor: page.next_cursor,
        })
    }
}
