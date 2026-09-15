use super::{decode, inconsistent, port};
use application::case_stages::*;
use application::ApplicationError;
use domain::cases::CaseId;
use domain::crypto::DocumentHasher;
use postgres::GenericClient;

pub(crate) const CHANGE_SELECT:&str="SELECT s.*,to_char(s.act_date,'YYYY-MM-DD') AS act_date_text,to_char(s.received_date,'YYYY-MM-DD') AS received_date_text,a.values_digest AS administration_digest,(a.nuc IS NOT NULL AND a.administrative_status='active') AS administration_valid,case_stage_values_canonical(s) AS canonical,case_stage_recording_valid(s) AS recording_valid FROM case_stage_revisions s JOIN case_administration_revisions a ON a.case_id=s.case_id AND a.revision=s.administration_revision";

pub(super) fn current<C: GenericClient>(
    tx: &mut C,
    case: CaseId,
    hasher: &dyn DocumentHasher,
) -> Result<CurrentCaseStage, ApplicationError> {
    let page = history(tx, case, &CaseStageQuery::new(1, None)?, hasher)?;
    Ok(page
        .entries
        .into_iter()
        .next()
        .map(|entry| CurrentCaseStage::Registered(Box::new(entry)))
        .unwrap_or(CurrentCaseStage::Unregistered))
}

pub(super) fn history<C: GenericClient>(
    tx: &mut C,
    case: CaseId,
    query: &CaseStageQuery,
    hasher: &dyn DocumentHasher,
) -> Result<CaseStagePage, ApplicationError> {
    let limit = i64::from(query.limit()) + 1;
    let before = query
        .before_revision()
        .map(|revision| i64::from(revision.get()));
    let mut keys=tx.query("SELECT revision,initial FROM (SELECT revision,FALSE AS initial FROM case_stage_revisions WHERE case_id=$1 UNION ALL SELECT stage_revision,TRUE FROM case_initial_stage_registrations WHERE case_id=$1) h WHERE ($2::bigint IS NULL OR revision<$2) ORDER BY revision DESC LIMIT $3",&[&case.as_uuid(),&before,&limit]).map_err(port)?;
    if keys
        .windows(2)
        .any(|pair| pair[0].get::<_, i64>(0) == pair[1].get::<_, i64>(0))
    {
        return Err(inconsistent("duplicate logical stage revision"));
    }
    let has_more = keys.len() > query.limit() as usize;
    keys.truncate(query.limit() as usize);
    let revisions: Vec<i64> = keys
        .iter()
        .filter(|row| !row.get::<_, bool>(1))
        .map(|row| row.get(0))
        .collect();
    let mut changed = if revisions.is_empty() {
        Vec::new()
    } else {
        tx.query(
            &format!("{CHANGE_SELECT} WHERE s.case_id=$1 AND s.revision=ANY($2::bigint[])"),
            &[&case.as_uuid(), &revisions],
        )
        .map_err(port)?
        .into_iter()
        .map(|row| {
            decode::changed(&row, hasher).map(|value| CaseStageEntry::Changed(Box::new(value)))
        })
        .collect::<Result<Vec<_>, _>>()?
    };
    let mut entries = Vec::with_capacity(keys.len());
    for key in keys {
        if key.get::<_, bool>(1) {
            entries.push(CaseStageEntry::Initial(initial(tx, case)?));
        } else {
            let revision = key.get::<_, i64>(0);
            let index = changed
                .iter()
                .position(|entry| i64::from(entry.stage_revision().get()) == revision)
                .ok_or_else(|| inconsistent("stage history context is missing"))?;
            entries.push(changed.remove(index));
        }
    }
    let next_before_revision = if has_more {
        entries.last().map(CaseStageEntry::stage_revision)
    } else {
        None
    };
    Ok(CaseStagePage {
        entries,
        has_more,
        next_before_revision,
    })
}

fn initial<C: GenericClient>(
    tx: &mut C,
    case: CaseId,
) -> Result<application::cases::CaseInitialStageRegistration, ApplicationError> {
    let row=tx.query_opt("SELECT s.case_id,s.stage_revision,s.administration_revision,s.stage,a.values_digest AS administration_digest,a.changed_at AS recorded_at_text,a.changed_by AS recorded_by,a.changed_by_email AS recorded_by_email,CASE WHEN a.nuc IS NOT NULL AND a.administrative_status='active' AND case_administration_is_canonical(a.administrative_status,a.title,a.reference,a.nuc,a.nuc_authority,a.judicial_case_number,a.judicial_authority,a.offenses,a.general_information,a.complementary_identifiers) THEN a.values_digest=sha256(case_administration_bytes(a.administrative_status,a.title,a.reference,a.nuc,a.nuc_authority,a.judicial_case_number,a.judicial_authority,a.offenses,a.general_information,a.complementary_identifiers)) ELSE FALSE END AS canonical FROM case_initial_stage_registrations s JOIN case_administration_revisions a ON a.case_id=s.case_id AND a.revision=s.administration_revision WHERE s.case_id=$1",&[&case.as_uuid()]).map_err(port)?.ok_or_else(||inconsistent("initial stage context is missing"))?;
    decode::initial(&row)
}
