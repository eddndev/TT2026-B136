use super::header;
use application::{
    deadline_reevaluation::{DependencyFamily, TechnicalCause, TechnicalService},
    deadlines::*,
    ApplicationError,
};
use serde_json::{json, Value};

pub(super) fn submission(value: &DeadlineDetail) -> Result<Value, ApplicationError> {
    let receipt = &value.receipt;
    let mut view = json!({
        "case_id":value.case_id.to_string(),"deadline_id":value.id.to_string(),
        "operation_id":receipt.operation_id.to_string(),"action":receipt.action.as_str(),
        "expected_revision":receipt.expected_revision,"review_digest":receipt.review_digest.to_hex(),
        "reason":value.reason.as_ref().map(|reason|reason.as_str())
    });
    match &receipt.version {
        DeadlineReceiptVersion::Legacy => {
            let (actor, _) = header::legacy_actor(&value.recorded_by)?;
            view["actor_id"] = json!(actor.to_string());
        }
        DeadlineReceiptVersion::Tracked(metadata) => {
            view["version"] = json!(2);
            view["observations_digest"] = json!(metadata.observations_digest.to_hex());
            view["predecessor"] = json!(metadata.predecessor.map(|previous| json!({
                "submission_digest":previous.submission_digest.to_hex(),
                "capture_digest":previous.capture_digest.to_hex(),
            })));
            view["author"] = match &value.recorded_by {
                DeadlineActorSnapshot::User { id, email } => json!({
                    "kind":"user","id":id.to_string(),"email":email,
                }),
                DeadlineActorSnapshot::Technical {
                    service: TechnicalService::DeadlineReevaluator,
                    policy_version,
                } => json!({
                    "kind":"technical","service":"deadline_reevaluator","policy_version":policy_version,
                }),
            };
            view["cause"] = metadata.cause.map_or(Value::Null, cause);
        }
    }
    Ok(view)
}

fn cause(value: TechnicalCause) -> Value {
    match value {
        TechnicalCause::LegacyBootstrap {
            job_id,
            policy_version,
        } => json!({
            "kind":"legacy_bootstrap","job_id":job_id.to_string(),"policy_version":policy_version,
        }),
        TechnicalCause::SourceEvent { job_id, event } => json!({
            "kind":"source_event","job_id":job_id.to_string(),"event":{
                "sequence":event.sequence,"family":match event.family {
                    DependencyFamily::Resolution=>"resolution",DependencyFamily::Notification=>"notification",
                    DependencyFamily::HearingResult=>"hearing_result",DependencyFamily::Calendar=>"calendar",
                    DependencyFamily::Profile=>"profile",
                },
                "source_id":event.source_id.to_string(),"revision":event.revision,
                "case_id":event.case_id.map(|id|id.to_string()),
                "hearing_id":event.hearing_id.map(|id|id.to_string()),
                "operation_id":event.operation_id.to_string(),
            }
        }),
    }
}
