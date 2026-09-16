use application::cases::*;
use application::identity::Principal;
use application::ApplicationError;
use domain::cases::{CaseId, CaseMetadata};
use domain::crypto::DocumentHasher;
use domain::identity::UserId;
use postgres::Transaction;
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

use super::values::{canonical_time, decode, inconsistent};

pub(crate) fn detail(
    tx: &mut Transaction<'_>,
    id: CaseId,
    hasher: &dyn DocumentHasher,
) -> Result<CaseAdministrationDetail, ApplicationError> {
    let row=tx.query_opt("SELECT id,title,reference,created_by,required_initial_revision,to_char(created_at AT TIME ZONE 'UTC','YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') AS created_at_text FROM cases WHERE id=$1", &[&id.as_uuid()]).map_err(port)?.ok_or(ApplicationError::CaseNotFound)?;
    let origin = CaseOrigin {
        id,
        created_by: UserId::from_uuid(row.get("created_by")),
        created_at: OffsetDateTime::parse(row.get("created_at_text"), &Rfc3339)
            .map_err(inconsistent)?,
    };
    let revision=tx.query_opt("SELECT * FROM case_administration_revisions WHERE case_id=$1 ORDER BY revision DESC LIMIT 1", &[&id.as_uuid()]).map_err(port)?;
    let administration = match revision {
        Some(row) => CurrentCaseAdministration::Recorded(Box::new(decode(&row, hasher)?)),
        None => {
            let required: Option<i64> = row.get("required_initial_revision");
            if required.is_some() {
                return Err(inconsistent("new case has no initial revision"));
            }
            let title: &str = row.get("title");
            let reference: &str = row.get("reference");
            let metadata = CaseMetadata::new(title, reference).map_err(inconsistent)?;
            if metadata.title() != title || metadata.reference() != reference {
                return Err(inconsistent("noncanonical baseline metadata"));
            }
            CurrentCaseAdministration::Unrevised(metadata)
        }
    };
    let stage=tx.query_opt("SELECT r.*,s.stage_revision,s.administration_revision,s.stage FROM case_initial_stage_registrations s JOIN case_administration_revisions r ON r.case_id=s.case_id AND r.revision=s.administration_revision WHERE s.case_id=$1", &[&id.as_uuid()]).map_err(port)?;
    let initial_stage = stage
        .map(|row| {
            let snapshot = decode(&row, hasher)?;
            if row.get::<_, i64>("stage_revision") != 1
                || row.get::<_, i64>("administration_revision") != 1
                || row.get::<_, &str>("stage") != "investigation"
            {
                return Err(inconsistent("invalid initial stage registration"));
            }
            Ok(CaseInitialStageRegistration {
                case_id: id,
                stage_revision: CaseStageRevision::FIRST,
                administration_revision: CaseRevision::FIRST,
                stage: InitialCaseStage::Investigation,
                administration_digest: snapshot.values_digest,
                recorded_at: snapshot.changed_at,
                recorded_by: snapshot.changed_by,
            })
        })
        .transpose()?;
    Ok(CaseAdministrationDetail {
        origin,
        administration,
        initial_stage,
    })
}

pub(super) fn snapshot(
    id: CaseId,
    revision: CaseRevision,
    values: CaseAdministrationValues,
    principal: &Principal,
    at: OffsetDateTime,
    hasher: &dyn DocumentHasher,
) -> CaseAdministrationSnapshot {
    CaseAdministrationSnapshot {
        case_id: id,
        revision,
        values_digest: case_administration_digest(hasher, &values),
        values,
        changed_at: at.to_offset(UtcOffset::UTC),
        changed_by: CaseActorSnapshot {
            id: principal.id,
            email: principal.email.clone(),
        },
    }
}

pub(super) fn insert(
    tx: &mut Transaction<'_>,
    s: &CaseAdministrationSnapshot,
) -> Result<(), ApplicationError> {
    let v = &s.values;
    let p = v.profile();
    tx.execute("INSERT INTO case_administration_revisions(case_id,revision,title,reference,administrative_status,nuc,nuc_authority,judicial_case_number,judicial_authority,offenses,general_information,complementary_identifiers,values_digest,changed_at,changed_by,changed_by_email) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)", &[&s.case_id.as_uuid(),&i64::from(s.revision.get()),&v.metadata().title(),&v.metadata().reference(),&v.status().as_str(),&p.map(PenalCaseProfile::nuc),&p.map(PenalCaseProfile::nuc_authority),&p.map(PenalCaseProfile::judicial_case_number),&p.map(PenalCaseProfile::judicial_authority),&p.map(PenalCaseProfile::offenses),&p.and_then(PenalCaseProfile::general_information),&p.and_then(PenalCaseProfile::complementary_identifiers),&&s.values_digest.as_bytes()[..],&canonical_time(s.changed_at)?,&s.changed_by.id.as_uuid(),&s.changed_by.email]).map_err(port)?;
    Ok(())
}
pub(super) fn basic(d: &CaseAdministrationDetail) -> CaseRecord {
    let values = d.administration.values();
    CaseRecord {
        id: d.origin.id,
        title: values.metadata().title().into(),
        reference: values.metadata().reference().into(),
        created_by: d.origin.created_by,
    }
}
pub(super) fn overview(d: CaseAdministrationDetail) -> CaseAdministrationOverview {
    let values = d.administration.values();
    CaseAdministrationOverview {
        origin: d.origin,
        metadata: values.metadata().clone(),
        revision: d.administration.revision(),
        administrative_status: values.status(),
        penal_identifiers: values.profile().map(|p| CasePenalIdentifiers {
            nuc: p.nuc().into(),
            judicial_case_number: p.judicial_case_number().into(),
        }),
        initial_stage: d.initial_stage.map(|s| s.stage),
    }
}
pub(super) fn resource(s: &CaseAdministrationSnapshot) -> String {
    format!(
        "case:{}:administration:revision:{}:sha256:{}",
        s.case_id,
        s.revision.get(),
        s.values_digest.to_hex()
    )
}
pub(super) fn current_resource(d: &CaseAdministrationDetail) -> String {
    d.administration
        .snapshot()
        .map(resource)
        .unwrap_or_else(|| format!("case:{}:administration:revision:0", d.origin.id))
}
pub(super) fn port(error: postgres::Error) -> ApplicationError {
    if error.as_db_error().is_some_and(|e| {
        e.code().code() == "23505"
            && matches!(
                e.constraint(),
                Some("case_current_nuc_unique" | "case_current_judicial_case_number_unique")
            )
    }) {
        return ApplicationError::CaseIdentifierConflict;
    }
    ApplicationError::Port(format!("case database: {error}"))
}
