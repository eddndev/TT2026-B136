use super::{inconsistent, sources, storage};
use crate::procedural_resource_codec as codec;
use application::{
    case_stages::{CaseStageQuery, CurrentCaseStage},
    cases::CaseActorSnapshot,
    procedural_facts::FactText,
    procedural_resources::*,
    ApplicationError,
};
use domain::{
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
    identity::UserId,
};
use postgres::{Row, Transaction};
use time::OffsetDateTime;

fn digest(row: &Row, key: &str) -> Result<Sha256Digest, ApplicationError> {
    let bytes: Vec<u8> = row.try_get(key).map_err(inconsistent)?;
    Ok(Sha256Digest::from_array(bytes.try_into().map_err(
        |_| inconsistent("resource digest length differs"),
    )?))
}
pub(super) fn detail(
    tx: &mut Transaction<'_>,
    row: &Row,
    hasher: &dyn DocumentHasher,
) -> Result<ResourceDetail, ApplicationError> {
    let case = CaseId::from_uuid(row.try_get("case_id").map_err(inconsistent)?);
    let revision = storage::revision(row.try_get("revision").map_err(inconsistent)?)?;
    let values = codec::decode_values(&row.try_get("values_view").map_err(inconsistent)?)?;
    if values.canonical_bytes()
        != row
            .try_get::<_, Vec<u8>>("values_canonical")
            .map_err(inconsistent)?
        || values.kind().as_str() != row.try_get::<_, String>("kind").map_err(inconsistent)?
    {
        return Err(inconsistent("resource values projection differs"));
    }
    let supports = codec::decode_supports(&row.try_get("supports_view").map_err(inconsistent)?)?;
    let sources =
        sources::reconstruct(tx, case, &values, supports, hasher).map_err(|error| match error {
            ApplicationError::ProceduralFact(
                application::procedural_facts::ProceduralFactError::NotFound
                | application::procedural_facts::ProceduralFactError::ReferenceNotFound,
            )
            | ApplicationError::ParticipantNotFound => {
                inconsistent("resource exact historical source is absent")
            }
            other => other,
        })?;
    if resource_sources_bytes(&sources)?
        != row
            .try_get::<_, Vec<u8>>("sources_canonical")
            .map_err(inconsistent)?
    {
        return Err(inconsistent("resource exact sources differ"));
    }
    let recorded_administration =
        crate::procedural_fact_postgres::administration::captured(tx, row, case, hasher)?;
    let stage: Option<i64> = row
        .try_get("recorded_stage_revision")
        .map_err(inconsistent)?;
    let recorded_stage = if let Some(stage) = stage {
        let number = u32::try_from(stage).map_err(inconsistent)?;
        let page = crate::case_stages::query::history(
            tx,
            case,
            &CaseStageQuery::new(1, number.checked_add(1))?,
            hasher,
        )?;
        let entry = page
            .entries
            .into_iter()
            .next()
            .filter(|s| s.stage_revision().get() == number)
            .ok_or_else(|| inconsistent("resource captured stage is absent"))?;
        CurrentCaseStage::Registered(Box::new(entry))
    } else {
        CurrentCaseStage::Unregistered
    };
    let seconds: i64 = row.try_get("recorded_at_seconds").map_err(inconsistent)?;
    let nanos: i32 = row
        .try_get("recorded_at_nanoseconds")
        .map_err(inconsistent)?;
    if !(0..1_000_000_000).contains(&nanos) {
        return Err(inconsistent("resource timestamp nanos differ"));
    }
    let at = OffsetDateTime::from_unix_timestamp(seconds)
        .map_err(inconsistent)?
        .replace_nanosecond(nanos as u32)
        .map_err(inconsistent)?;
    if !(1..=9999).contains(&at.year()) {
        return Err(inconsistent("resource timestamp exceeds supported years"));
    }
    if recorded_administration
        .snapshot()
        .is_some_and(|a| at < a.changed_at)
        || recorded_stage.entry().is_some_and(|s| at < s.recorded_at())
    {
        return Err(inconsistent("resource timestamp predates captured context"));
    }
    let action = action(&row.try_get::<_, String>("action").map_err(inconsistent)?)?;
    let previous = if revision.get() == 1 {
        None
    } else {
        Some(ResourceRevisionRef {
            revision: ResourceRevision::new(revision.get() - 1)?,
            capture_digest: digest(row, "previous_capture_digest")?,
        })
    };
    let result = ResourceDetail {
        case_id: case,
        id: ResourceId::from_uuid(row.try_get("resource_id").map_err(inconsistent)?),
        revision,
        values,
        status: row
            .try_get::<_, String>("status")
            .map_err(inconsistent)?
            .parse()
            .map_err(inconsistent)?,
        sources,
        act: act(row, tx, case)?,
        reason: row
            .try_get::<_, Option<String>>("reason")
            .map_err(inconsistent)?
            .map(|s| {
                let text = FactText::new(&s).map_err(inconsistent)?;
                if text.as_str() != s {
                    return Err(inconsistent("resource reason is not canonical"));
                }
                Ok(text)
            })
            .transpose()?,
        receipt: ResourceReceipt {
            operation_id: ResourceOperationId::from_uuid(
                row.try_get("operation_id").map_err(inconsistent)?,
            ),
            action,
            expected_revision: revision.get() - 1,
            previous,
            values_digest: digest(row, "values_digest")?,
            sources_digest: digest(row, "sources_digest")?,
            submission_digest: digest(row, "submission_digest")?,
            capture_digest: digest(row, "capture_digest")?,
        },
        recorded_by: CaseActorSnapshot {
            id: UserId::from_uuid(row.try_get("recorded_by").map_err(inconsistent)?),
            email: row.try_get("recorded_by_email").map_err(inconsistent)?,
        },
        recorded_at: at,
        recorded_administration,
        recorded_stage,
    };
    resource_receipt_matches(hasher, &result)?;
    let draft = ResourceDraft {
        case_id: case,
        command: resource_command_from_detail(&result)?,
        result_revision: revision,
        values: result.values.clone(),
        status: result.status,
        sources: result.sources.clone(),
        act: result.act.clone(),
        previous: result.receipt.previous,
        recorded_by: result.recorded_by.clone(),
        observed_administration: result.recorded_administration.clone(),
        observed_stage: result.recorded_stage.clone(),
        submission_digest: result.receipt.submission_digest,
    };
    if resource_submission_bytes(hasher, &draft)?
        != row
            .try_get::<_, Vec<u8>>("submission_canonical")
            .map_err(inconsistent)?
        || resource_capture_bytes(&result)
            != row
                .try_get::<_, Vec<u8>>("capture_canonical")
                .map_err(inconsistent)?
    {
        return Err(inconsistent("resource receipt bytes differ"));
    }
    Ok(result)
}
fn act(
    row: &Row,
    tx: &mut Transaction<'_>,
    case: CaseId,
) -> Result<Option<ResourceActCapture>, ApplicationError> {
    let id: Option<uuid::Uuid> = row.try_get("act_id").map_err(inconsistent)?;
    let Some(id) = id else { return Ok(None) };
    let values = codec::decode_act(&row.try_get("act_values_view").map_err(inconsistent)?)?;
    if values.canonical_bytes()
        != row
            .try_get::<_, Vec<u8>>("act_values_canonical")
            .map_err(inconsistent)?
    {
        return Err(inconsistent("resource act values differ"));
    }
    let supports =
        codec::decode_supports(&row.try_get("act_supports_view").map_err(inconsistent)?)?;
    sources::validate_supports(tx, case, &supports)?;
    let previous: Option<i64> = row
        .try_get("act_previous_resource_revision")
        .map_err(inconsistent)?;
    Ok(Some(ResourceActCapture {
        id: ResourceActId::from_uuid(id),
        revision: ResourceActRevision::new(
            u32::try_from(
                row.try_get::<_, i64>("act_revision")
                    .map_err(inconsistent)?,
            )
            .map_err(inconsistent)?,
        )?,
        values,
        supports,
        previous: previous
            .map(|r| {
                Ok::<_, ApplicationError>(ResourceRevisionRef {
                    revision: storage::revision(r)?,
                    capture_digest: digest(row, "act_previous_capture_digest")?,
                })
            })
            .transpose()?,
    }))
}
pub(super) fn action(value: &str) -> Result<ResourceAction, ApplicationError> {
    match value {
        "register" => Ok(ResourceAction::Register),
        "correct" => Ok(ResourceAction::Correct),
        "record_act" => Ok(ResourceAction::RecordAct),
        "correct_act" => Ok(ResourceAction::CorrectAct),
        "archive" => Ok(ResourceAction::Archive),
        "reactivate" => Ok(ResourceAction::Reactivate),
        _ => Err(inconsistent("unknown resource action")),
    }
}
