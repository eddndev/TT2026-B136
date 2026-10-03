use application::identity::password_reset::ResetDeliveryOutcome;
use reqwest::blocking::Response;
use serde::Deserialize;
use std::io::Read;
use zeroize::Zeroizing;

const MAX_RESPONSE_BYTES: usize = 8192;

#[derive(Deserialize)]
struct Receipt<'a> {
    #[serde(borrow)]
    id: Option<&'a str>,
    #[serde(borrow)]
    name: Option<&'a str>,
}

pub(super) fn classify(response: Response) -> ResetDeliveryOutcome {
    let status = response.status().as_u16();
    let mut bytes = Zeroizing::new(Vec::new());
    if response
        .take((MAX_RESPONSE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .is_err()
        || bytes.len() > MAX_RESPONSE_BYTES
    {
        return ResetDeliveryOutcome::Uncertain;
    }
    // Provider receipts are objects; Serde structs also accept positional arrays.
    if bytes.iter().find(|byte| !byte.is_ascii_whitespace()) != Some(&b'{') {
        return ResetDeliveryOutcome::Uncertain;
    }
    let Ok(receipt) = serde_json::from_slice::<Receipt<'_>>(&bytes) else {
        return ResetDeliveryOutcome::Uncertain;
    };
    if matches!(status, 200 | 201) && receipt.name.is_none() {
        if receipt.id.is_some_and(|id| {
            uuid::Uuid::parse_str(id).is_ok_and(|value| !value.is_nil() && value.to_string() == id)
        }) {
            return ResetDeliveryOutcome::Accepted;
        }
    } else if receipt.id.is_none() && definite_rejection(status, receipt.name) {
        return ResetDeliveryOutcome::DefinitelyRejected;
    }
    ResetDeliveryOutcome::Uncertain
}

fn definite_rejection(status: u16, name: Option<&str>) -> bool {
    matches!(
        (status, name),
        (400, Some("validation_error" | "invalid_idempotency_key"))
            | (401, Some("missing_api_key" | "restricted_api_key"))
            | (
                403,
                Some(
                    "invalid_permission"
                        | "restricted_api_key"
                        | "suspended_api_key"
                        | "validation_error"
                )
            )
            | (404, Some("not_found"))
            | (405, Some("method_not_allowed"))
            | (
                422,
                Some(
                    "invalid_attachment"
                        | "invalid_parameter"
                        | "missing_required_field"
                        | "missing_required_parameter"
                )
            )
            | (
                429,
                Some("daily_quota_exceeded" | "monthly_quota_exceeded" | "rate_limit_exceeded")
            )
    )
}
