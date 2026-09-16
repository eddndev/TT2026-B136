use application::cases::*;
use application::ApplicationError;
use domain::cases::CaseMetadata;
use domain::crypto::{DocumentHasher, Sha256Digest};
use domain::identity::UserId;
use postgres::Row;
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

pub(crate) fn decode(
    row: &Row,
    hasher: &dyn DocumentHasher,
) -> Result<CaseAdministrationSnapshot, ApplicationError> {
    let title: String = row.try_get("title").map_err(inconsistent)?;
    let reference: String = row.try_get("reference").map_err(inconsistent)?;
    let metadata = CaseMetadata::new(&title, &reference).map_err(inconsistent)?;
    if metadata.title() != title || metadata.reference() != reference {
        return Err(inconsistent("noncanonical case metadata"));
    }
    let nuc: Option<String> = row.try_get("nuc").map_err(inconsistent)?;
    let nuc_authority: Option<String> = row.try_get("nuc_authority").map_err(inconsistent)?;
    let number: Option<String> = row.try_get("judicial_case_number").map_err(inconsistent)?;
    let authority: Option<String> = row.try_get("judicial_authority").map_err(inconsistent)?;
    let offenses: Option<Vec<String>> = row.try_get("offenses").map_err(inconsistent)?;
    let general: Option<String> = row.try_get("general_information").map_err(inconsistent)?;
    let complementary: Option<String> = row
        .try_get("complementary_identifiers")
        .map_err(inconsistent)?;
    let profile = if let Some(nuc) = nuc.as_deref() {
        let refs = offenses
            .as_ref()
            .ok_or_else(|| inconsistent("missing offenses"))?
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let profile = PenalCaseProfile::new(
            nuc,
            required(&nuc_authority)?,
            required(&number)?,
            required(&authority)?,
            &refs,
            general.as_deref(),
            complementary.as_deref(),
        )
        .map_err(inconsistent)?;
        if profile.nuc() != nuc
            || profile.nuc_authority() != required(&nuc_authority)?
            || profile.judicial_case_number() != required(&number)?
            || profile.judicial_authority() != required(&authority)?
            || profile.offenses() != offenses.as_deref().unwrap_or_default()
            || profile.general_information() != general.as_deref()
            || profile.complementary_identifiers() != complementary.as_deref()
        {
            return Err(inconsistent("noncanonical penal profile"));
        }
        Some(profile)
    } else {
        if nuc_authority.is_some()
            || number.is_some()
            || authority.is_some()
            || offenses.is_some()
            || general.is_some()
            || complementary.is_some()
        {
            return Err(inconsistent("partial penal profile"));
        }
        None
    };
    let status: &str = row.try_get("administrative_status").map_err(inconsistent)?;
    let values = CaseAdministrationValues::new(
        CaseEditableValues::new(metadata, profile),
        status.parse().map_err(inconsistent)?,
    );
    let bytes: Vec<u8> = row.try_get("values_digest").map_err(inconsistent)?;
    let values_digest = Sha256Digest::from_array(
        bytes
            .try_into()
            .map_err(|_| inconsistent("invalid case digest length"))?,
    );
    if values_digest != case_administration_digest(hasher, &values) {
        return Err(inconsistent("case values digest mismatch"));
    }
    let revision: i64 = row.try_get("revision").map_err(inconsistent)?;
    let timestamp: String = row.try_get("changed_at").map_err(inconsistent)?;
    let changed_at = OffsetDateTime::parse(&timestamp, &Rfc3339).map_err(inconsistent)?;
    if canonical_time(changed_at)? != timestamp {
        return Err(inconsistent("noncanonical case revision timestamp"));
    }
    let email: String = row.try_get("changed_by_email").map_err(inconsistent)?;
    if email.is_empty() || email.trim() != email || email.chars().any(char::is_control) {
        return Err(inconsistent("noncanonical case actor email"));
    }
    Ok(CaseAdministrationSnapshot {
        case_id: domain::cases::CaseId::from_uuid(row.try_get("case_id").map_err(inconsistent)?),
        revision: CaseRevision::new(u32::try_from(revision).map_err(inconsistent)?)
            .map_err(inconsistent)?,
        values,
        values_digest,
        changed_at,
        changed_by: CaseActorSnapshot {
            id: UserId::from_uuid(row.try_get("changed_by").map_err(inconsistent)?),
            email,
        },
    })
}
fn required(value: &Option<String>) -> Result<&str, ApplicationError> {
    value
        .as_deref()
        .ok_or_else(|| inconsistent("incomplete penal profile"))
}
pub(super) fn canonical_time(at: OffsetDateTime) -> Result<String, ApplicationError> {
    at.to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .map_err(inconsistent)
}
pub(super) fn inconsistent(error: impl std::fmt::Display) -> ApplicationError {
    ApplicationError::StoredCaseAdministrationInconsistent(error.to_string())
}
