use super::{inconsistent, port};
use application::{
    identity::Principal, resource_activities::*, resource_hearings::*, ApplicationError,
};
use domain::{
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
    identity::{Role, UserId},
    procedural_resources::{ResourceActId, ResourceActRevision},
    resource_hearings::*,
};
use postgres::{Row, Transaction};

pub(crate) fn detail(
    tx: &mut Transaction<'_>,
    case: CaseId,
    id: ResourceHearingId,
    revision: Option<ResourceHearingRevision>,
    hasher: &dyn DocumentHasher,
) -> Result<ResourceHearingDetail, ApplicationError> {
    let revision = revision.map(|v| i64::from(v.get()));
    let probe = tx
        .query_opt(
            "SELECT revision,octet_length(values_view::text)<=65536
        AND octet_length(values_canonical)<=65536 AND octet_length(submission_canonical)<=1048576
        AND octet_length(capture_canonical)<=1048576 AND octet_length(recorded_by_email)<=1280
        AS bounded FROM case_resource_hearing_revisions
        WHERE hearing_id=$1 AND case_id=$2 AND ($3::bigint IS NULL OR revision=$3)
        ORDER BY revision DESC LIMIT 1",
            &[&id.as_uuid(), &case.as_uuid(), &revision],
        )
        .map_err(port)?
        .ok_or(ResourceActivityError::NotFound)?;
    if !probe.get::<_, bool>("bounded") {
        return Err(inconsistent("resource hearing fields exceed bounds"));
    }
    let row = tx.query_one("SELECT * FROM case_resource_hearing_revisions
        WHERE hearing_id=$1 AND case_id=$2 AND revision=$3 AND octet_length(values_view::text)<=65536
        AND octet_length(values_canonical)<=65536 AND octet_length(submission_canonical)<=1048576
        AND octet_length(capture_canonical)<=1048576 AND octet_length(recorded_by_email)<=1280",
        &[&id.as_uuid(), &case.as_uuid(), &probe.get::<_,i64>("revision")]).map_err(port)?;
    decode(tx, &row, hasher)
}
fn decode(
    tx: &mut Transaction<'_>,
    row: &Row,
    hasher: &dyn DocumentHasher,
) -> Result<ResourceHearingDetail, ApplicationError> {
    let case = CaseId::from_uuid(row.get("case_id"));
    let resource = ResourceCaptureRef {
        id: ResourceId::from_uuid(row.get("resource_id")),
        revision: ResourceRevision::new(counter(row, "resource_revision")?)
            .map_err(inconsistent)?,
        capture_digest: digest(row, "resource_capture_digest")?,
    };
    let act = row
        .get::<_, Option<uuid::Uuid>>("act_id")
        .map(|id| {
            Ok::<_, ApplicationError>(ResourceActCaptureRef {
                id: ResourceActId::from_uuid(id),
                revision: ResourceActRevision::new(counter(row, "act_revision")?)
                    .map_err(inconsistent)?,
                resource_revision: ResourceRevision::new(counter(row, "act_resource_revision")?)
                    .map_err(inconsistent)?,
                capture_digest: digest(row, "act_capture_digest")?,
            })
        })
        .transpose()?;
    if act.is_none()
        && (row.get::<_, Option<i64>>("act_revision").is_some()
            || row.get::<_, Option<i64>>("act_resource_revision").is_some()
            || row
                .get::<_, Option<Vec<u8>>>("act_capture_digest")
                .is_some())
    {
        return Err(inconsistent("resource hearing act fields are incomplete"));
    }
    let values = crate::resource_hearing_codec::values(
        &row.get::<_, Vec<u8>>("values_canonical"),
        &row.get("values_view"),
    )?;
    let expected_resource_revision =
        ResourceRevision::new(counter(row, "recorded_resource_revision")?).map_err(inconsistent)?;
    let command = ResourceHearingCommand {
        operation_id: ResourceHearingOperationId::from_uuid(row.get("operation_id")),
        hearing_id: ResourceHearingId::from_uuid(row.get("hearing_id")),
        association_id: ResourceActivityId::from_uuid(row.get("association_id")),
        expected_resource_revision,
        resource,
        act,
        values,
    };
    let (source, captured_act) = crate::resource_activity_postgres::sources::load_resource(
        tx,
        case,
        resource.id,
        resource,
        act,
        hasher,
    )?;
    let head = crate::procedural_resource_postgres::storage::detail(
        tx,
        case,
        resource.id,
        Some(expected_resource_revision),
        hasher,
    )?;
    if head.receipt.capture_digest != digest(row, "recorded_resource_capture_digest")? {
        return Err(inconsistent("resource hearing historical head differs"));
    }
    let mut participants = Vec::new();
    for reference in command.values.participants() {
        participants.push(crate::participant_postgres::storage::exact(
            tx,
            case,
            reference.id(),
            reference.revision(),
            hasher,
        )?);
    }
    let material = ResourceHearingMaterial {
        case_id: case,
        administration: crate::procedural_fact_postgres::administration::captured(
            tx, row, case, hasher,
        )?,
        resource_head: head,
        resource: source,
        act: captured_act,
        participants,
    };
    let actor = Principal {
        id: UserId::from_uuid(row.get("recorded_by")),
        email: row.get("recorded_by_email"),
        role: Role::Owner,
    };
    let review = prepare_resource_hearing_review(
        hasher,
        &actor,
        case,
        resource.id,
        command,
        material.clone(),
    )?;
    if review.submission_digest != digest(row, "submission_digest")?
        || resource_hearing_submission_bytes(hasher, &review)?
            != row.get::<_, Vec<u8>>("submission_canonical")
    {
        return Err(inconsistent("resource hearing submission differs"));
    }
    let seconds: i64 = row.get("recorded_at_seconds");
    let nanos: i32 = row.get("recorded_at_nanoseconds");
    let recorded_at = time::OffsetDateTime::from_unix_timestamp(seconds)
        .map_err(inconsistent)?
        .replace_nanosecond(u32::try_from(nanos).map_err(inconsistent)?)
        .map_err(inconsistent)?;
    let detail = ResourceHearingDetail {
        review,
        material,
        revision: ResourceHearingRevision::new(counter(row, "revision")?).map_err(inconsistent)?,
        recorded_at,
        capture_digest: digest(row, "capture_digest")?,
    };
    resource_hearing_receipt_matches(hasher, &detail)?;
    if resource_hearing_capture_bytes(&detail) != row.get::<_, Vec<u8>>("capture_canonical") {
        return Err(inconsistent("resource hearing capture bytes differ"));
    }
    Ok(detail)
}
fn counter(row: &Row, key: &str) -> Result<u32, ApplicationError> {
    u32::try_from(row.try_get::<_, i64>(key).map_err(inconsistent)?).map_err(inconsistent)
}
fn digest(row: &Row, key: &str) -> Result<Sha256Digest, ApplicationError> {
    let bytes: Vec<u8> = row.try_get(key).map_err(inconsistent)?;
    Ok(Sha256Digest::from_array(bytes.try_into().map_err(
        |_| inconsistent("resource hearing digest length differs"),
    )?))
}
