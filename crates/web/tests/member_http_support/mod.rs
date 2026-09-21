use application::{members::*, ApplicationError};
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use domain::{
    cases::CaseId,
    identity::{Role, UserId},
};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;
use uuid::Uuid;

pub struct Workflow(pub Mutex<Vec<String>>);

pub fn user_id() -> UserId {
    UserId::from_uuid(Uuid::from_u128(7))
}
pub fn case_id() -> CaseId {
    CaseId::from_uuid(Uuid::from_u128(8))
}
pub fn user() -> UserSummary {
    UserSummary {
        id: user_id(),
        email: "staff@example.com".into(),
        role: Role::Paralegal,
        active: true,
        revision: 9007199254740993,
    }
}
impl Workflow {
    pub fn new() -> Arc<Self> {
        Arc::new(Self(Mutex::new(Vec::new())))
    }
    fn authorize(&self, token: &str, operation: &str) -> Result<(), ApplicationError> {
        self.0.lock().unwrap().push(operation.into());
        match token {
            "owner" => Ok(()),
            "expired" => Err(ApplicationError::InvalidSession),
            "unavailable" => Err(ApplicationError::Port("internal database detail".into())),
            _ => Err(ApplicationError::PermissionDenied),
        }
    }
}
impl MemberWorkflow for Workflow {
    fn list(&self, token: &str, query: UserQuery) -> Result<UserPage, ApplicationError> {
        self.authorize(token, "list")?;
        assert_eq!(query.limit(), 1);
        assert_eq!(query.status(), UserStatusFilter::All);
        assert_eq!(query.role(), Some(Role::Paralegal));
        assert_eq!(query.email_prefix(), Some("staff"));
        Ok(UserPage {
            items: vec![user()],
            has_more: true,
            next_cursor: Some(query.cursor_after(user_id())),
        })
    }
    fn get(&self, token: &str, id: UserId) -> Result<UserSummary, ApplicationError> {
        self.authorize(token, "get")?;
        if id != user_id() {
            return Err(ApplicationError::UserNotFound);
        }
        Ok(user())
    }
    fn change_access(
        &self,
        token: &str,
        id: UserId,
        change: UserAccessChange,
    ) -> Result<UserSummary, ApplicationError> {
        self.authorize(token, "change")?;
        if id != user_id() {
            return Err(ApplicationError::UserNotFound);
        }
        match change.expected_revision() {
            0 => return Err(MemberError::RevisionConflict.into()),
            1 => return Err(MemberError::LastActiveOwner.into()),
            2 => return Err(MemberError::AccessVersionExhausted.into()),
            3 => return Err(MemberError::Stored("hidden field detail").into()),
            9007199254740993 => (),
            _ => panic!("unexpected revision"),
        }
        Ok(UserSummary {
            revision: 9007199254740994,
            role: change.role(),
            active: change.active(),
            ..user()
        })
    }
    fn list_case_members(
        &self,
        token: &str,
        case: CaseId,
        query: CaseMemberQuery,
    ) -> Result<CaseMemberPage, ApplicationError> {
        self.authorize(token, "members")?;
        assert_eq!(case, case_id());
        assert_eq!(query.case_id(), case);
        assert_eq!(query.limit(), 1);
        assert_eq!(query.role(), Some(Role::Paralegal));
        assert_eq!(query.email_prefix(), Some("staff"));
        let assigned = query.selection() == MemberSelection::Assigned;
        Ok(CaseMemberPage {
            case_id: case,
            items: vec![CaseMemberItem {
                user: UserSummary {
                    active: !assigned,
                    ..user()
                },
                assigned_at: assigned.then_some(time::OffsetDateTime::UNIX_EPOCH),
            }],
            has_more: false,
            next_cursor: None,
        })
    }
}

pub async fn request(
    workflow: &Arc<Workflow>,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<serde_json::Value>,
) -> axum::response::Response {
    let mut builder = Request::builder()
        .method(method)
        .uri(format!("/api/v1{path}"));
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    let bytes = if let Some(body) = body {
        builder = builder.header("content-type", "application/json");
        serde_json::to_vec(&body).unwrap()
    } else {
        Vec::new()
    };
    web::member_router(workflow.clone())
        .oneshot(builder.body(Body::from(bytes)).unwrap())
        .await
        .unwrap()
}
pub async fn json(response: axum::response::Response) -> serde_json::Value {
    serde_json::from_slice(&to_bytes(response.into_body(), 32768).await.unwrap()).unwrap()
}
