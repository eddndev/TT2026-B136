use super::{decode, inconsistent, port, sources};
use application::{case_stages::StageSupportSnapshot, hearing_results::*, ApplicationError};
use domain::{
    case_administration::{CaseAdministrativeStatus, CaseRevision},
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
    hearings::HearingId,
};
use postgres::Transaction;

type Stored = (HearingResultSnapshot, Option<StageSupportSnapshot>);

pub(crate) fn detail(
    tx: &mut Transaction<'_>,
    case: CaseId,
    hearing: HearingId,
    id: HearingResultId,
    revision: Option<HearingResultRevision>,
    hasher: &dyn DocumentHasher,
) -> Result<HearingResultDetail, ApplicationError> {
    let (snapshot, support) = raw(tx, case, hearing, id, revision, hasher)?;
    let (anchor, attendees) = validate_sources(tx, &snapshot, &support, hasher)?;
    validate_previous(tx, &snapshot, &support, hasher)?;
    let continuation = snapshot
        .continuation
        .map(|reference| {
            let prior = self::snapshot(
                tx,
                case,
                reference.hearing_id,
                reference.result_id,
                reference.revision,
                hasher,
            )
            .map_err(stored_source)?;
            administration_follows(
                &snapshot,
                prior.recorded_administration_revision,
                prior.recorded_administration_digest,
            )?;
            let projection = HearingResultContinuationSnapshot::from(&prior);
            if projection.reference != reference {
                return Err(inconsistent("result continuation source differs"));
            }
            Ok(projection)
        })
        .transpose()?;
    let detail = HearingResultDetail {
        snapshot,
        anchor,
        continuation,
        attendees,
        support,
    };
    hearing_result_receipt_matches(hasher, &detail)?;
    Ok(detail)
}
/// Resolves one exact antecedent without recursively traversing its continuation.
pub(crate) fn snapshot(
    tx: &mut Transaction<'_>,
    case: CaseId,
    hearing: HearingId,
    id: HearingResultId,
    revision: HearingResultRevision,
    hasher: &dyn DocumentHasher,
) -> Result<HearingResultSnapshot, ApplicationError> {
    let (snapshot, support) = raw(tx, case, hearing, id, Some(revision), hasher)?;
    validate_sources(tx, &snapshot, &support, hasher)?;
    validate_previous(tx, &snapshot, &support, hasher)?;
    Ok(snapshot)
}
fn validate_sources(
    tx: &mut Transaction<'_>,
    snapshot: &HearingResultSnapshot,
    support: &Option<StageSupportSnapshot>,
    hasher: &dyn DocumentHasher,
) -> Result<
    (
        HearingResultAnchorSnapshot,
        Vec<HearingResultAttendeeSnapshot>,
    ),
    ApplicationError,
> {
    let anchor = sources::anchor(
        tx,
        snapshot.case_id,
        snapshot.hearing_id,
        snapshot.anchor.revision,
        hasher,
    )
    .map_err(stored_source)?;
    administration_follows(
        snapshot,
        anchor.snapshot.recorded_administration_revision,
        anchor.snapshot.recorded_administration_digest,
    )?;
    let anchor = HearingResultAnchorSnapshot::from(&anchor);
    if anchor.reference != snapshot.anchor {
        return Err(inconsistent("result anchor source differs"));
    }
    let administration = crate::hearing_postgres::administration(
        tx,
        snapshot.case_id,
        snapshot.recorded_administration_revision,
        hasher,
    )?;
    if administration.values_digest != snapshot.recorded_administration_digest
        || administration.values.status() != CaseAdministrativeStatus::Active
    {
        return Err(inconsistent(
            "result recorded administration source differs",
        ));
    }
    let attendees = sources::attendees(tx, snapshot.case_id, &snapshot.values, hasher)
        .map_err(stored_source)?;
    sources::support(tx, snapshot.case_id, support)?;
    Ok((anchor, attendees))
}
fn validate_previous(
    tx: &mut Transaction<'_>,
    snapshot: &HearingResultSnapshot,
    support: &Option<StageSupportSnapshot>,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    if snapshot.revision.get() == 1 {
        return Ok(());
    }
    let previous = HearingResultRevision::new(snapshot.revision.get() - 1).map_err(inconsistent)?;
    let (prior, old_support) = raw(
        tx,
        snapshot.case_id,
        snapshot.hearing_id,
        snapshot.id,
        Some(previous),
        hasher,
    )
    .map_err(stored_source)?;
    administration_follows(
        snapshot,
        prior.recorded_administration_revision,
        prior.recorded_administration_digest,
    )?;
    if prior.status != HearingResultStatus::Recorded
        || prior.anchor != snapshot.anchor
        || prior.continuation != snapshot.continuation
    {
        return Err(inconsistent(
            "result history does not preserve its recorded predecessor and sources",
        ));
    }
    if snapshot.receipt.action == HearingResultAction::Withdraw
        && (prior.values != snapshot.values || &old_support != support)
    {
        return Err(inconsistent(
            "result withdrawal changed declared content or support",
        ));
    }
    Ok(())
}
fn raw(
    tx: &mut Transaction<'_>,
    case: CaseId,
    hearing: HearingId,
    id: HearingResultId,
    revision: Option<HearingResultRevision>,
    hasher: &dyn DocumentHasher,
) -> Result<Stored, ApplicationError> {
    let revision = revision.map(|r| i64::from(r.get()));
    let probe=tx.query_opt("SELECT revision,octet_length(values_canonical) BETWEEN 26 AND 146933
        AND octet_length(submission_canonical) BETWEEN 192 AND 4296
        AND octet_length(values_view::text)<=1048576 AND octet_length(submission_view::text)<=32768
        AND octet_length(recorded_by_email)<=1280 AND COALESCE(octet_length(reason),0)<=4000
        AND COALESCE(octet_length(support_name),0)<=128 AS bounded
        FROM case_hearing_result_revisions WHERE result_id=$1 AND case_id=$2 AND hearing_id=$3 AND ($4::bigint IS NULL OR revision=$4)
        ORDER BY revision DESC LIMIT 1",&[&id.as_uuid(),&case.as_uuid(),&hearing.as_uuid(),&revision]).map_err(port)?.ok_or(HearingResultError::NotFound)?;
    if !probe.get::<_, bool>("bounded") {
        return Err(inconsistent("result persisted fields exceed bounds"));
    }
    let selected: i64 = probe.get("revision");
    let row=tx.query_opt("SELECT r.*,h.initial_revision,h.anchor_revision,h.anchor_values_digest,h.anchor_submission_digest,
        h.continuation_hearing_id,h.continuation_result_id,h.continuation_revision,h.continuation_values_digest,h.continuation_submission_digest
        FROM case_hearing_result_revisions r JOIN case_hearing_results h ON h.id=r.result_id AND h.case_id=r.case_id AND h.hearing_id=r.hearing_id
        WHERE r.result_id=$1 AND r.case_id=$2 AND r.hearing_id=$3 AND r.revision=$4
        AND octet_length(r.values_canonical) BETWEEN 26 AND 146933 AND octet_length(r.submission_canonical) BETWEEN 192 AND 4296
        AND octet_length(r.values_view::text)<=1048576 AND octet_length(r.submission_view::text)<=32768
        AND octet_length(r.recorded_by_email)<=1280 AND COALESCE(octet_length(r.reason),0)<=4000 AND COALESCE(octet_length(r.support_name),0)<=128",
        &[&id.as_uuid(),&case.as_uuid(),&hearing.as_uuid(),&selected]).map_err(port)?.ok_or_else(||inconsistent("result immutable revision or root changed during read"))?;
    decode::row(&row, hasher)
}
fn administration_follows(
    snapshot: &HearingResultSnapshot,
    revision: CaseRevision,
    digest: Sha256Digest,
) -> Result<(), ApplicationError> {
    if snapshot.recorded_administration_revision < revision
        || (snapshot.recorded_administration_revision == revision
            && snapshot.recorded_administration_digest != digest)
    {
        return Err(inconsistent(
            "result recorded administration predates or contradicts an exact source",
        ));
    }
    Ok(())
}
fn stored_source(error: ApplicationError) -> ApplicationError {
    match error {
        ApplicationError::HearingResult(
            HearingResultError::NotFound | HearingResultError::ReferenceNotFound,
        ) => inconsistent("exact result historical source is absent"),
        other => other,
    }
}
