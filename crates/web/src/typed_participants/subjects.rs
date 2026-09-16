use super::{
    body, parse_case, parse_revision, parse_uuid, projection,
    review::{Candidate, Review},
    values::SubjectValues,
    ParticipantState,
};
use crate::{error::ApiError, request::bearer_token};
use application::{typed_participants as m, ApplicationError};
use axum::{
    extract::{rejection::QueryRejection, Path, Query, Request, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

pub(super) fn router() -> Router<ParticipantState> {
    Router::new()
        .route("/api/v1/cases/:case/subjects", get(list))
        .route("/api/v1/cases/:case/subjects/:id", get(detail).put(replace))
        .route("/api/v1/cases/:case/subjects/:id/review", post(review))
        .route(
            "/api/v1/cases/:case/subjects/:id/revisions/:revision",
            get(revision),
        )
        .route("/api/v1/cases/:case/subjects/:id/history", get(history))
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Kind {
    NaturalPerson,
    InstitutionalBody,
}
impl From<Kind> for m::SubjectKind {
    fn from(v: Kind) -> Self {
        match v {
            Kind::NaturalPerson => Self::NaturalPerson,
            Kind::InstitutionalBody => Self::InstitutionalBody,
        }
    }
}
#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct List {
    limit: u32,
    after_id: Option<Uuid>,
    name: Option<String>,
    kind: Option<Kind>,
}
impl Default for List {
    fn default() -> Self {
        Self {
            limit: 50,
            after_id: None,
            name: None,
            kind: None,
        }
    }
}
#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct History {
    limit: u32,
    before_revision: Option<u32>,
}
impl Default for History {
    fn default() -> Self {
        Self {
            limit: 50,
            before_revision: None,
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReviewRequest {
    expected_revision: u32,
    values: SubjectValues,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Replacement {
    expected_revision: u32,
    values: SubjectValues,
    review: Review,
}

async fn list(
    State(s): State<ParticipantState>,
    Path(case): Path<String>,
    headers: HeaderMap,
    input: Result<Query<List>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let case = parse_case(&case)?;
    let Query(q) = input.map_err(query_error)?;
    let q = m::SubjectQuery::new(
        q.limit,
        q.after_id.map(m::CaseSubjectId::from_uuid),
        q.name.as_deref(),
        q.kind.map(Into::into),
    )?;
    let row = s
        .runtime
        .run(move || s.workflow.list_subjects(&token, case, q))
        .await?;
    Ok(Json(
        json!({"subjects":row.subjects.into_iter().map(|v|json!({"case_id":v.case_id.to_string(),"id":v.id.to_string(),"revision":v.revision.get(),"kind":v.kind.as_str(),"display_name":v.display_name})).collect::<Vec<_>>(),
        "has_more":row.has_more,"next_after_id":row.next_after_id.map(|v|v.to_string())}),
    ))
}
async fn detail(
    State(s): State<ParticipantState>,
    Path((case, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let (case, id) = scope(&case, &id)?;
    let row = s
        .runtime
        .run(move || s.workflow.get_subject(&token, case, id))
        .await?;
    Ok(Json(projection::subject(&row)?))
}
async fn revision(
    State(s): State<ParticipantState>,
    Path((case, id, revision)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let (case, id) = scope(&case, &id)?;
    let revision =
        m::SubjectRevision::new(parse_revision(&revision)?).map_err(ApplicationError::from)?;
    let row = s
        .runtime
        .run(move || s.workflow.get_subject_revision(&token, case, id, revision))
        .await?;
    Ok(Json(projection::subject(&row)?))
}
async fn history(
    State(s): State<ParticipantState>,
    Path((case, id)): Path<(String, String)>,
    headers: HeaderMap,
    input: Result<Query<History>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let (case, id) = scope(&case, &id)?;
    let Query(q) = input.map_err(query_error)?;
    let revision = q
        .before_revision
        .map(m::SubjectRevision::new)
        .transpose()
        .map_err(ApplicationError::from)?;
    let q = m::SubjectHistoryQuery::new(q.limit, revision)?;
    let row = s
        .runtime
        .run(move || s.workflow.subject_history(&token, case, id, q))
        .await?;
    Ok(Json(
        json!({"revisions":row.revisions.iter().map(projection::subject).collect::<Result<Vec<_>,_>>()?,"has_more":row.has_more,
        "next_before_revision":row.next_before_revision.map(|v|v.get())}),
    ))
}
async fn review(
    State(s): State<ParticipantState>,
    Path((case, id)): Path<(String, String)>,
    headers: HeaderMap,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let (case, id) = scope(&case, &id)?;
    let input = body::json::<ReviewRequest>(request).await?;
    let expected =
        m::SubjectRevision::new(input.expected_revision).map_err(ApplicationError::from)?;
    let values = input.values.validate()?;
    let row = s
        .runtime
        .run(move || {
            s.workflow
                .review_subject(&token, case, id, expected, values)
        })
        .await?;
    Ok(Json(
        json!({"case_id":case.to_string(),"id":row.id.to_string(),"expected_revision":row.expected_revision.get(),
        "values":SubjectValues::from(&row.values),"directory_stamp":row.directory_stamp.0.to_hex(),"candidates":row.candidates.into_iter().map(Candidate::from).collect::<Vec<_>>()}),
    ))
}
async fn replace(
    State(s): State<ParticipantState>,
    Path((case, id)): Path<(String, String)>,
    headers: HeaderMap,
    request: Request,
) -> Result<Json<Value>, ApiError> {
    let token = bearer_token(&headers)?;
    let (case, id) = scope(&case, &id)?;
    let input = body::json::<Replacement>(request).await?;
    let input = m::SubjectReplacementRequest {
        id,
        expected_revision: m::SubjectRevision::new(input.expected_revision)
            .map_err(ApplicationError::from)?,
        values: input.values.validate()?,
        review: input.review.validate()?,
    };
    let row = s
        .runtime
        .run(move || s.workflow.replace_subject(&token, case, input))
        .await?;
    Ok(Json(projection::subject(&row)?))
}
fn scope(case: &str, id: &str) -> Result<(domain::cases::CaseId, m::CaseSubjectId), ApiError> {
    Ok((
        parse_case(case)?,
        m::CaseSubjectId::from_uuid(parse_uuid(id, "invalid_subject_id")?),
    ))
}
fn query_error(_: QueryRejection) -> ApiError {
    ApiError::invalid_body("invalid_query", "invalid subject query parameters")
}
