use super::{inconsistent, port, HearingProofRef, HistoryRoot};
use crate::measure_administrative_postgres::{decode, inventory::audit};
use application::{
    identity::Principal, measure_corrections::MeasureAdministrativeError,
    precautionary_hearings::*, ApplicationError,
};
use domain::{
    case_administration::{CaseRevision, CaseStageRevision},
    cases::CaseId,
    crypto::DocumentHasher,
    hearings::HearingNote,
    identity::{Role, UserId},
    precautionary_hearings::*,
};
use postgres::{Row, Transaction};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

type References = BTreeSet<(Uuid, u32, [u8; 32])>;

pub(super) fn roots(
    tx: &mut Transaction<'_>,
    case: CaseId,
    target: PrecautionaryMeasureRef,
    hasher: &dyn DocumentHasher,
) -> Result<(Vec<HistoryRoot>, References), ApplicationError> {
    crate::precautionary_hearing_postgres::audit::case_intact(tx, case)?;
    let orphan: bool = tx
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM case_precautionary_hearing_revisions r
        LEFT JOIN case_precautionary_hearings h ON h.id=r.hearing_id AND h.case_id=r.case_id
        WHERE r.case_id=$1 AND h.id IS NULL)",
            &[&case.as_uuid()],
        )
        .map_err(port)?
        .get(0);
    if orphan {
        return Err(inconsistent("hearing declaration has no same-case root"));
    }
    let mut selected = BTreeMap::new();
    let mut references = BTreeSet::new();
    let mut after: Option<Uuid> = None;
    loop {
        let rows = tx
            .query(
                "SELECT id FROM case_precautionary_hearings
            WHERE case_id=$1 AND ($2::uuid IS NULL OR id>$2) ORDER BY id LIMIT 8",
                &[&case.as_uuid(), &after],
            )
            .map_err(port)?;
        if rows.is_empty() {
            break;
        }
        for row in rows {
            let id: Uuid = row.get(0);
            let prefix = crate::precautionary_hearing_postgres::storage::prefix(
                tx,
                case,
                PrecautionaryHearingId::from_uuid(id),
                None,
            )?;
            let mut previous: Option<Declaration> = None;
            for row in prefix.rows {
                let declaration = declaration(tx, case, &row, previous.as_ref(), hasher)?;
                if declaration.values.review_targets().contains(&target) {
                    references.insert((
                        id,
                        declaration.reference.revision.get(),
                        *declaration.reference.capture_digest.as_bytes(),
                    ));
                    selected.insert(id, declaration.reference);
                    if references.len() > 256 {
                        return Err(MeasureAdministrativeError::IncompleteHistory.into());
                    }
                }
                previous = Some(declaration);
            }
            after = Some(id);
        }
    }
    Ok((
        selected.into_values().map(HistoryRoot::Hearing).collect(),
        references,
    ))
}

struct Declaration {
    reference: HearingProofRef,
    values: PrecautionaryHearingValues,
    cancelled: bool,
    at: time::OffsetDateTime,
    sequence: i64,
}

fn declaration(
    tx: &mut Transaction<'_>,
    case: CaseId,
    row: &Row,
    previous: Option<&Declaration>,
    hasher: &dyn DocumentHasher,
) -> Result<Declaration, ApplicationError> {
    if row.get::<_, Uuid>("case_id") != case.as_uuid() {
        return Err(inconsistent("hearing declaration belongs to another case"));
    }
    let id = PrecautionaryHearingId::from_uuid(row.get("hearing_id"));
    let revision =
        PrecautionaryHearingRevision::new(integer(row, "revision")?).map_err(inconsistent)?;
    let action: String = row.get("action");
    let old = row
        .get::<_, Option<Vec<u8>>>("previous_capture_digest")
        .map(decode::digest)
        .transpose()?;
    if previous.is_some_and(|previous| previous.cancelled || previous.reference.hearing_id != id)
        || old != previous.map(|previous| previous.reference.capture_digest)
    {
        return Err(inconsistent("hearing declaration predecessor differs"));
    }
    let context = PrecautionaryContextExpectation {
        administration_revision: CaseRevision::new(integer(
            row,
            "observed_administration_revision",
        )?)
        .map_err(inconsistent)?,
        stage_revision: CaseStageRevision::new(integer(row, "observed_stage_revision")?)
            .map_err(inconsistent)?,
        context_digest: decode::digest(row.get("observed_context_digest"))?,
    };
    let values = if action == "cancel" {
        if row.get::<_, Option<Vec<u8>>>("values_canonical").is_some()
            || row
                .get::<_, Option<serde_json::Value>>("values_view")
                .is_some()
            || row.get::<_, Option<Vec<u8>>>("values_digest").is_some()
            || row.get::<_, Option<String>>("support_format").is_some()
            || row.get::<_, Option<String>>("support_policy").is_some()
        {
            return Err(inconsistent("cancellation declares new values or support"));
        }
        previous
            .ok_or_else(|| inconsistent("cancellation has no prior values"))?
            .values
            .clone()
    } else {
        let bytes = row
            .get::<_, Option<Vec<u8>>>("values_canonical")
            .ok_or_else(|| inconsistent("hearing declaration values are absent"))?;
        let view = row
            .get::<_, Option<serde_json::Value>>("values_view")
            .ok_or_else(|| inconsistent("hearing declaration projection is absent"))?;
        let digest = decode::digest(
            row.get::<_, Option<Vec<u8>>>("values_digest")
                .ok_or_else(|| inconsistent("hearing values commitment is absent"))?,
        )?;
        if hasher.hash_bytes(&bytes) != digest
            || !matches!(
                row.get::<_, Option<String>>("support_format").as_deref(),
                Some("pdf" | "docx")
            )
            || row.get::<_, Option<String>>("support_policy").as_deref() != Some("pdf_docx_v1")
        {
            return Err(inconsistent(
                "hearing declaration values or support commitments differ",
            ));
        }
        crate::precautionary_hearing_codec::values(&bytes, &view)?
    };
    let change = match (action.as_str(), previous) {
        ("schedule", None) if row.get::<_, Option<String>>("reason").is_none() => {
            PrecautionaryHearingChange::Schedule {
                context,
                values: values.clone(),
            }
        }
        ("replace", Some(previous)) => PrecautionaryHearingChange::Replace {
            expected_revision: previous.reference.revision,
            expected_capture_digest: previous.reference.capture_digest,
            context,
            values: values.clone(),
            reason: reason(row)?,
        },
        ("cancel", Some(previous)) => PrecautionaryHearingChange::Cancel {
            expected_revision: previous.reference.revision,
            expected_capture_digest: previous.reference.capture_digest,
            reason: reason(row)?,
        },
        _ => {
            return Err(inconsistent(
                "hearing declaration action or position differs",
            ))
        }
    };
    let command = PrecautionaryHearingCommand {
        operation_id: PrecautionaryHearingOperationId::from_uuid(row.get("operation_id")),
        hearing_id: id,
        change,
    };
    if command.result_revision()? != revision {
        return Err(inconsistent(
            "hearing declaration revision differs from command",
        ));
    }
    let actor = Principal {
        id: UserId::from_uuid(row.get("recorded_by")),
        email: row.get("recorded_by_email"),
        role: row
            .get::<_, String>("recorded_by_role")
            .parse()
            .map_err(inconsistent)?,
    };
    let submission = decode::digest(row.get("submission_digest"))?;
    if !matches!(actor.role, Role::Owner | Role::Litigator)
        || hasher.hash_bytes(&precautionary_hearing_submission_bytes(
            &actor, case, &command, &values,
        )?) != submission
    {
        return Err(inconsistent(
            "hearing declaration differs from original submission",
        ));
    }
    let capture = decode::digest(row.get("capture_digest"))?;
    let review = decode::digest(row.get("review_digest"))?;
    let at = audit::time(row)?;
    let sequence: i64 = row.get("audit_sequence");
    if previous.is_some_and(|previous| previous.at > at || previous.sequence >= sequence) {
        return Err(inconsistent(
            "hearing declaration clock or audit sequence regressed",
        ));
    }
    let marker = format!(
        "ph1:case:{case}:hearing:{id}:operation:{}:revision:{}:submission:{}:review:{}:capture:{}",
        command.operation_id,
        revision.get(),
        submission.to_hex(),
        review.to_hex(),
        capture.to_hex(),
    );
    audit::verify(
        tx,
        sequence,
        &actor.email,
        &format!("precautionary_hearing.{action}"),
        &marker,
        at,
        hasher,
    )?;
    Ok(Declaration {
        reference: HearingProofRef {
            hearing_id: id,
            revision,
            capture_digest: capture,
        },
        values,
        cancelled: action == "cancel",
        at,
        sequence,
    })
}

fn integer(row: &Row, name: &str) -> Result<u32, ApplicationError> {
    u32::try_from(row.get::<_, i64>(name)).map_err(inconsistent)
}
fn reason(row: &Row) -> Result<HearingNote, ApplicationError> {
    let text = row
        .get::<_, Option<String>>("reason")
        .ok_or_else(|| inconsistent("hearing declaration reason is absent"))?;
    let reason = HearingNote::new(&text).map_err(inconsistent)?;
    if reason.as_str() != text {
        return Err(inconsistent("hearing declaration reason is not canonical"));
    }
    Ok(reason)
}
