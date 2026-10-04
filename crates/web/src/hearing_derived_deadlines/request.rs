use crate::{deadlines, error::ApiError, hearing_results, procedural_facts::object::Object};
use application::{
    deadlines::DeadlineChange, hearing_derived_deadlines::HearingDerivedDeadlineCommand,
    hearing_results::HearingResultChange,
};
use domain::{
    cases::CaseId, crypto::Sha256Digest, deadline_triggers::TriggerSourceRef, hearings::HearingId,
    procedural_facts::FactDeclaration,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Command {
    case_id: String,
    result: Object<hearing_results::CommandInput>,
    deadline: Object<deadlines::CommandInput>,
}

impl Command {
    pub(super) fn validate(
        self,
        case: CaseId,
        hearing: HearingId,
    ) -> Result<HearingDerivedDeadlineCommand, ApiError> {
        if uuid(&self.case_id)? != case.as_uuid() {
            return Err(invalid());
        }
        let result = self.result.0.validate()?;
        let deadline = self.deadline.0.validate()?;
        let (value, _) = deadline.clone().into_parts();
        let DeadlineChange::Register { definition } = value.change else {
            return Err(invalid());
        };
        let selection = &definition.input.selection;
        let FactDeclaration::Known(TriggerSourceRef::HearingResult(source)) = selection.source
        else {
            return Err(invalid());
        };
        if result.hearing_id != hearing
            || !matches!(result.change, HearingResultChange::Record { .. })
            || selection.case_id != case
            || source.hearing_id != hearing
            || source.result_id != result.result_id
            || source.revision.get() != 1
        {
            return Err(invalid());
        }
        Ok(HearingDerivedDeadlineCommand { result, deadline })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Submission {
    command: Object<Command>,
    expected_review_digest: String,
}

impl Submission {
    pub(super) fn validate(
        self,
        case: CaseId,
        hearing: HearingId,
    ) -> Result<(HearingDerivedDeadlineCommand, Sha256Digest), ApiError> {
        let digest = &self.expected_review_digest;
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid());
        }
        Ok((
            self.command.0.validate(case, hearing)?,
            Sha256Digest::from_hex(digest).map_err(|_| invalid())?,
        ))
    }
}

pub(super) fn uuid(value: &str) -> Result<uuid::Uuid, ApiError> {
    let id = uuid::Uuid::parse_str(value).map_err(|_| invalid())?;
    if id.to_string() != value {
        return Err(invalid());
    }
    Ok(id)
}

pub(super) fn invalid() -> ApiError {
    ApiError::invalid_body(
        "invalid_hearing_derived_deadline",
        "invalid hearing derived deadline command",
    )
}
