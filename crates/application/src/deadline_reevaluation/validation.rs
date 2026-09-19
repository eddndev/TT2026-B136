use super::*;
use domain::{cases::CaseId, procedural_facts::FactText, typed_participants::Uuid};

type Result<T> = std::result::Result<T, TrackingCodecError>;

pub(super) fn scope(
    case: CaseId,
    family: DependencyFamily,
    scoped_case: Option<CaseId>,
    hearing: Option<Uuid>,
) -> Result<()> {
    let valid = match family {
        DependencyFamily::Profile => scoped_case.is_none_or(|id| id == case) && hearing.is_none(),
        DependencyFamily::Calendar => scoped_case.is_none() && hearing.is_none(),
        DependencyFamily::Resolution | DependencyFamily::Notification => {
            scoped_case == Some(case) && hearing.is_none()
        }
        DependencyFamily::HearingResult => scoped_case == Some(case) && hearing.is_some(),
    };
    if valid {
        Ok(())
    } else {
        Err(TrackingCodecError::InvalidShape("dependency scope"))
    }
}

pub(super) fn submission(value: &TrackedSubmission) -> Result<()> {
    let register = value.action == TrackedAction::Register;
    let technical = value.action == TrackedAction::Reevaluate;
    if (register
        && (value.expected_revision != 0 || value.predecessor.is_some() || value.reason.is_some()))
        || (!register
            && (value.expected_revision == 0
                || value.expected_revision == u32::MAX
                || value.predecessor.is_none()
                || value.reason.is_none()))
    {
        return Err(TrackingCodecError::InvalidShape("action and predecessor"));
    }
    if let Some(reason) = &value.reason {
        if !FactText::new(reason).is_ok_and(|text| text.as_str() == reason) {
            return Err(TrackingCodecError::InvalidShape("canonical reason"));
        }
    }
    match &value.author {
        TrackedAuthor::User { email, .. } => {
            if technical
                || value.cause.is_some()
                || email.is_empty()
                || email.trim() != email
                || email.chars().count() > 320
                || email.chars().any(char::is_control)
            {
                return Err(TrackingCodecError::InvalidShape("user author"));
            }
        }
        TrackedAuthor::Technical { policy_version, .. } => {
            if !technical || *policy_version != 1 || value.cause.is_none() {
                return Err(TrackingCodecError::InvalidShape("technical author"));
            }
        }
    }
    if let Some(cause) = value.cause {
        technical_cause(value.case_id, cause)?;
    }
    Ok(())
}

pub(crate) fn technical_cause(case_id: CaseId, cause: TechnicalCause) -> Result<()> {
    match cause {
        TechnicalCause::LegacyBootstrap { policy_version, .. } => {
            if policy_version != 1 {
                return Err(TrackingCodecError::InvalidShape("bootstrap policy"));
            }
        }
        TechnicalCause::SourceEvent { event, .. } => {
            if event.sequence == 0 || event.sequence > i64::MAX as u64 || event.revision == 0 {
                return Err(TrackingCodecError::InvalidShape(
                    "source event revision or sequence",
                ));
            }
            scope(case_id, event.family, event.case_id, event.hearing_id)?;
        }
    }
    Ok(())
}
