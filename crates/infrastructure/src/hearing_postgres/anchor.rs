use super::{anchor_audit, decode, inconsistent, port, storage};
use application::{hearings::*, ApplicationError};
use domain::{
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
};
use postgres::Transaction;
use std::collections::BTreeSet;

/// Resolve one exact Initial anchor and its complete ordinary receipt prefix.
/// The caller retains transaction ownership and authorization responsibility.
pub(crate) fn load_initial_hearing_anchor(
    tx: &mut Transaction<'_>,
    case: CaseId,
    id: HearingId,
    revision: HearingRevision,
    values_digest: Sha256Digest,
    submission_digest: Sha256Digest,
    hasher: &dyn DocumentHasher,
) -> Result<HearingDetail, ApplicationError> {
    if revision.get() > 256 {
        return Err(inconsistent("initial anchor prefix exceeds 256 revisions"));
    }
    let root = tx
        .query_opt(
            "SELECT id FROM case_hearings WHERE id=$1 AND case_id=$2 AND initial_revision=1",
            &[&id.as_uuid(), &case.as_uuid()],
        )
        .map_err(port)?;
    if root.is_none() {
        return Err(inconsistent("initial anchor root is absent or foreign"));
    }
    let rows = tx
        .query(
            "SELECT revision,case_id,COALESCE(
         octet_length(values_canonical) BETWEEN 27 AND 10726
         AND octet_length(submission_canonical) BETWEEN 108 AND 4120
         AND octet_length(values_view::text)<=65536
         AND octet_length(submission_view::text)<=32768
         AND octet_length(values_digest)=32 AND octet_length(submission_digest)=32
         AND octet_length(scheduling_administration_digest)=32
         AND octet_length(recorded_administration_digest)=32
         AND COALESCE(octet_length(scheduling_stage_digest),32)=32
         AND octet_length(recorded_by_email) BETWEEN 1 AND 1280
         AND COALESCE(octet_length(reason),0)<=4000
         AND octet_length(action)<=8 AND octet_length(status)<=9
         AND octet_length(scheduling_stage)<=13
         AND COALESCE(octet_length(support_name),0)<=128
         AND COALESCE(octet_length(support_format),0)<=4
         AND COALESCE(octet_length(support_policy),0)<=11
         AND values_view->>'kind'='initial'
         AND recorded_at_seconds BETWEEN -62135596800 AND 253402300799
         AND recorded_at_nanoseconds BETWEEN 0 AND 999999999,FALSE) AS bounded
         FROM case_hearing_revisions WHERE hearing_id=$1 AND revision<=$2
         ORDER BY revision LIMIT 257",
            &[&id.as_uuid(), &i64::from(revision.get())],
        )
        .map_err(port)?;
    if rows.len() != revision.get() as usize {
        return Err(inconsistent("initial anchor prefix is incomplete"));
    }
    for (index, row) in rows.iter().enumerate() {
        if row.get::<_, i64>("revision") != index as i64 + 1
            || row.get::<_, uuid::Uuid>("case_id") != case.as_uuid()
            || !row.get::<_, bool>("bounded")
        {
            return Err(inconsistent(
                "initial anchor prefix is foreign, unbounded or unordered",
            ));
        }
    }
    let mut previous: Option<HearingDetail> = None;
    let mut previous_audit = None;
    let mut operations = BTreeSet::new();
    for row in rows {
        let selected =
            HearingRevision::new(decode::counter(row.get("revision"))?).map_err(inconsistent)?;
        bound_sources(tx, case, id, selected, hasher)?;
        let detail = storage::detail(tx, case, id, Some(selected), hasher)?;
        let snapshot = &detail.snapshot;
        if snapshot.case_id != case
            || snapshot.id != id
            || snapshot.revision != selected
            || snapshot.values.kind() != HearingKind::Initial
            || !operations.insert(snapshot.receipt.operation_id.as_uuid())
        {
            return Err(inconsistent(
                "initial anchor prefix identity or operation differs",
            ));
        }
        match &previous {
            None if snapshot.receipt.action == HearingAction::Schedule => {}
            Some(prior)
                if prior.snapshot.status == HearingStatus::Scheduled
                    && matches!(
                        snapshot.receipt.action,
                        HearingAction::Replace | HearingAction::Cancel
                    ) =>
            {
                if snapshot.receipt.action == HearingAction::Cancel
                    && (snapshot.values != prior.snapshot.values
                        || snapshot.scheduling_context != prior.snapshot.scheduling_context
                        || detail.participants != prior.participants
                        || detail.support != prior.support)
                {
                    return Err(inconsistent(
                        "initial cancellation changes retained programming",
                    ));
                }
            }
            _ => {
                return Err(inconsistent(
                    "initial anchor prefix action ancestry differs",
                ))
            }
        }
        let sequence = anchor_audit::verify(tx, &detail, hasher)?;
        if previous_audit.is_some_and(|prior| prior >= sequence) {
            return Err(inconsistent(
                "initial anchor audit ancestry is not ascending",
            ));
        }
        previous_audit = Some(sequence);
        previous = Some(detail);
    }
    let detail = previous.ok_or_else(|| inconsistent("initial anchor prefix is empty"))?;
    if detail.snapshot.values_digest != values_digest
        || detail.snapshot.receipt.submission_digest != submission_digest
    {
        return Err(inconsistent("initial anchor exact commitments differ"));
    }
    Ok(detail)
}

// Bound source transfers before the existing ordinary reconstruction reads them.
fn bound_sources(
    tx: &mut Transaction<'_>,
    case: CaseId,
    id: HearingId,
    revision: HearingRevision,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let row = tx.query_one(
        "SELECT * FROM case_hearing_revisions WHERE hearing_id=$1 AND case_id=$2 AND revision=$3",
        &[&id.as_uuid(), &case.as_uuid(), &i64::from(revision.get())],
    ).map_err(port)?;
    let detail = decode::row(&row, hasher)?;
    let snapshot = &detail.snapshot;
    for selected in [
        snapshot.scheduling_context.administration_revision.get(),
        snapshot.recorded_administration_revision.get(),
    ] {
        bound_administration(tx, case, i64::from(selected))?;
    }
    let stage = i64::from(snapshot.scheduling_context.stage_revision.get());
    let sources = tx
        .query(
            "SELECT administration_revision,TRUE AS bounded FROM case_initial_stage_registrations
         WHERE case_id=$1 AND stage_revision=$2 AND octet_length(stage)<=13
         UNION ALL SELECT administration_revision,COALESCE(
         octet_length(change_kind)<=15 AND COALESCE(octet_length(from_stage),0)<=13
         AND octet_length(stage)<=13 AND octet_length(act_precision)<=7
         AND COALESCE(octet_length(received_precision),0)<=7
         AND COALESCE(octet_length(reason),0)<=4000 AND COALESCE(octet_length(note),0)<=4000
         AND COALESCE(octet_length(receiving_court),0)<=800
         AND COALESCE(octet_length(receipt_reference),0)<=800
         AND octet_length(support_name)<=128 AND octet_length(support_digest)=32
         AND octet_length(support_format)<=4 AND octet_length(support_policy)<=11
         AND COALESCE(octet_length(receipt_name),0)<=128
         AND COALESCE(octet_length(receipt_digest),32)=32
         AND COALESCE(octet_length(receipt_format),0)<=4
         AND COALESCE(octet_length(receipt_policy),0)<=11
         AND octet_length(recorded_by_email) BETWEEN 1 AND 1280
         AND octet_length(values_digest)=32,FALSE) FROM case_stage_revisions
         WHERE case_id=$1 AND revision=$2 LIMIT 3",
            &[&case.as_uuid(), &stage],
        )
        .map_err(port)?;
    if sources.len() != 1 || !sources[0].get::<_, bool>("bounded") {
        return Err(inconsistent(
            "initial anchor stage source is absent, duplicated or unbounded",
        ));
    }
    bound_administration(tx, case, sources[0].get("administration_revision"))?;
    for reference in snapshot.values.participants() {
        crate::precautionary_hearing_postgres::sources::exact_participant(
            tx, case, *reference, hasher,
        )?;
    }
    Ok(())
}

fn bound_administration(
    tx: &mut Transaction<'_>,
    case: CaseId,
    revision: i64,
) -> Result<(), ApplicationError> {
    let found = tx
        .query_opt(
            "SELECT revision FROM case_administration_revisions WHERE case_id=$1 AND revision=$2
         AND octet_length(title)<=800 AND octet_length(reference)<=400
         AND octet_length(administrative_status)<=6 AND octet_length(nuc)<=400
         AND octet_length(nuc_authority)<=800 AND octet_length(judicial_case_number)<=400
         AND octet_length(judicial_authority)<=800 AND cardinality(offenses) BETWEEN 1 AND 8
         AND NOT EXISTS(SELECT 1 FROM unnest(offenses) item WHERE octet_length(item)>480)
         AND COALESCE(octet_length(general_information),0)<=4000
         AND COALESCE(octet_length(complementary_identifiers),0)<=1200
         AND octet_length(changed_at)<=64 AND octet_length(changed_by_email) BETWEEN 1 AND 1280
         AND octet_length(values_digest)=32",
            &[&case.as_uuid(), &revision],
        )
        .map_err(port)?;
    if found.is_none() {
        return Err(inconsistent(
            "initial anchor administration is absent or unbounded",
        ));
    }
    Ok(())
}
