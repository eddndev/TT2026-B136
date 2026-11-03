//! Contract workflow codes of `docs/adr/0086-bilateral-contract-signing.md`.
//! The individual signing codes the workflow reuses keep their own mapping.

use super::{ApiError, ErrorBody};
use application::{
    contracts::{ContractBlockers, ContractWorkflowError as Error},
    ApplicationError,
};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

pub(super) fn map(error: ApplicationError) -> Result<ApiError, ApplicationError> {
    let ApplicationError::Contract(error) = error else {
        return Err(error);
    };
    let (status, code) = match error {
        Error::NotFound => (StatusCode::NOT_FOUND, "contract_not_found"),
        Error::Exists => (StatusCode::CONFLICT, "contract_exists"),
        Error::RevisionConflict => (StatusCode::CONFLICT, "contract_revision_conflict"),
        Error::RequestOpen => (StatusCode::CONFLICT, "contract_request_open"),
        Error::RequestClosed => (StatusCode::CONFLICT, "contract_request_closed"),
        Error::OutOfTurn => (StatusCode::CONFLICT, "contract_out_of_turn"),
        Error::Expired => (StatusCode::CONFLICT, "contract_expired"),
        Error::VersionSuperseded => (StatusCode::CONFLICT, "contract_version_superseded"),
        Error::VersionFrozen => (StatusCode::CONFLICT, "contract_version_frozen"),
        Error::CounterpartyRevoked => (StatusCode::CONFLICT, "contract_counterparty_revoked"),
        Error::NotComplete => (StatusCode::CONFLICT, "contract_not_complete"),
        Error::PartyIneligible(_) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "contract_party_ineligible",
        ),
        Error::TimestampUnavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            "contract_timestamp_unavailable",
        ),
        Error::Input => (StatusCode::BAD_REQUEST, "invalid_contract_request"),
        Error::Inconsistent => return Ok(ApiError::internal()),
    };
    Ok(ApiError {
        status,
        code,
        message: error.to_string(),
    })
}

#[derive(Serialize)]
struct IneligibleEnvelope {
    error: IneligibleBody,
}

#[derive(Serialize)]
struct IneligibleBody {
    #[serde(flatten)]
    base: ErrorBody,
    blockers: Vec<&'static str>,
}

/// The `contract_party_ineligible` envelope with its stable blocker codes.
pub(crate) fn ineligible(blockers: ContractBlockers) -> Response {
    let base =
        map(Error::PartyIneligible(blockers).into()).unwrap_or_else(|_| ApiError::internal());
    let status = base.status;
    let body = IneligibleEnvelope {
        error: IneligibleBody {
            base: ErrorBody {
                code: base.code,
                message: base.message,
            },
            blockers: blockers.codes(),
        },
    };
    (status, Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code(error: Error) -> (u16, &'static str) {
        let mapped = ApiError::from(ApplicationError::from(error));
        (mapped.status.as_u16(), mapped.code)
    }

    #[test]
    fn every_contract_outcome_has_its_stable_status_and_code() {
        for (error, expected) in [
            (Error::NotFound, (404, "contract_not_found")),
            (Error::Exists, (409, "contract_exists")),
            (Error::RevisionConflict, (409, "contract_revision_conflict")),
            (Error::RequestOpen, (409, "contract_request_open")),
            (Error::RequestClosed, (409, "contract_request_closed")),
            (Error::OutOfTurn, (409, "contract_out_of_turn")),
            (Error::Expired, (409, "contract_expired")),
            (
                Error::VersionSuperseded,
                (409, "contract_version_superseded"),
            ),
            (Error::VersionFrozen, (409, "contract_version_frozen")),
            (
                Error::CounterpartyRevoked,
                (409, "contract_counterparty_revoked"),
            ),
            (Error::NotComplete, (409, "contract_not_complete")),
            (
                Error::PartyIneligible(ContractBlockers::default()),
                (422, "contract_party_ineligible"),
            ),
            (
                Error::TimestampUnavailable,
                (503, "contract_timestamp_unavailable"),
            ),
            (Error::Input, (400, "invalid_contract_request")),
            (Error::Inconsistent, (500, "internal_error")),
        ] {
            assert_eq!(code(error), expected, "{error:?}");
        }
    }

    #[tokio::test]
    async fn ineligibility_lists_only_its_blocker_codes() {
        let response = ineligible(ContractBlockers {
            client_not_ready: true,
            firm_signer_not_ready: false,
            document_not_ready: true,
        });
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let bytes = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["error"]["code"], "contract_party_ineligible");
        assert_eq!(
            value["error"]["blockers"],
            serde_json::json!(["client_not_ready", "document_not_ready"])
        );
        assert_eq!(value["error"].as_object().unwrap().len(), 3);
    }
}
