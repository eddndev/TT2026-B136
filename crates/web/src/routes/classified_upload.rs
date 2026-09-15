//! A bounded multipart request produces one atomic classified upload.

use axum::{
    body::{to_bytes, Body},
    extract::{
        multipart::{Field, MultipartError},
        FromRequest, Multipart, Path, Request, State,
    },
    http::{HeaderMap, StatusCode},
    Json,
};

use super::{
    documents::{parse_case_id, required_header},
    metadata::MAX_METADATA_BYTES,
    AppState, MAX_DOCUMENT_BYTES,
};
use crate::{
    dto::{DocumentOverviewResponse, MetadataValuesRequest},
    error::ApiError,
    request::bearer_token,
};

pub(super) const MAX_CLASSIFIED_UPLOAD_BYTES: usize = MAX_DOCUMENT_BYTES + 32 * 1024;

pub(super) async fn upload_with_metadata(
    State(state): State<AppState>,
    Path(case): Path<String>,
    headers: HeaderMap,
    request: Request,
) -> Result<(StatusCode, Json<DocumentOverviewResponse>), ApiError> {
    let token = bearer_token(&headers)?;
    let case = parse_case_id(&case)?;
    let name = required_header(&headers, "x-document-name")?.to_owned();
    let (parts, body) = request.into_parts();
    let bytes = to_bytes(body, MAX_CLASSIFIED_UPLOAD_BYTES)
        .await
        .map_err(body_error)?;
    let request = Request::from_parts(parts, Body::from(bytes));
    let mut multipart = Multipart::from_request(request, &())
        .await
        .map_err(|_| invalid_multipart())?;
    let mut file = None;
    let mut metadata = None;
    while let Some(field) = multipart.next_field().await.map_err(multipart_error)? {
        match field.name() {
            Some("file") if file.is_none() => {
                file = Some(read_part(field, MAX_DOCUMENT_BYTES, "document_too_large").await?);
            }
            Some("metadata") if metadata.is_none() => {
                let bytes = read_part(field, MAX_METADATA_BYTES, "metadata_too_large").await?;
                let input: MetadataValuesRequest =
                    serde_json::from_slice(&bytes).map_err(|_| {
                        ApiError::invalid_body("invalid_json", "invalid metadata JSON object")
                    })?;
                metadata = Some(input.validate()?);
            }
            _ => return Err(invalid_multipart()),
        }
    }
    let file = file.ok_or_else(invalid_multipart)?;
    let metadata = metadata.ok_or_else(invalid_multipart)?;
    let workflow = state.workflow.clone();
    let overview = state
        .runtime
        .run(move || workflow.upload_with_metadata(&token, case, &name, &file, metadata))
        .await?;
    Ok((StatusCode::CREATED, Json(overview.into())))
}

async fn read_part(
    mut field: Field<'_>,
    limit: usize,
    code: &'static str,
) -> Result<Vec<u8>, ApiError> {
    let mut bytes = Vec::new();
    while let Some(chunk) = field.chunk().await.map_err(multipart_error)? {
        if chunk.len() > limit - bytes.len() {
            return Err(ApiError::payload_too_large(
                code,
                "multipart part exceeds its allowed size",
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn invalid_multipart() -> ApiError {
    ApiError::invalid_body(
        "invalid_multipart",
        "exactly one complete file and metadata part are required",
    )
}

fn multipart_error(error: MultipartError) -> ApiError {
    if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        ApiError::payload_too_large(
            "upload_too_large",
            "multipart upload exceeds its allowed size",
        )
    } else {
        invalid_multipart()
    }
}

fn body_error(error: axum::Error) -> ApiError {
    if std::error::Error::source(&error)
        .is_some_and(|source| source.is::<http_body_util::LengthLimitError>())
    {
        ApiError::payload_too_large(
            "upload_too_large",
            "multipart upload exceeds its allowed size",
        )
    } else {
        invalid_multipart()
    }
}
