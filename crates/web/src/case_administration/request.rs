use application::cases::{
    CaseAdministrativeStatus, CaseEditableValues, CaseRevisionExpectation, PenalCaseCreation,
    PenalCaseProfile,
};
use application::ApplicationError;
use domain::cases::CaseMetadata;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProfileData {
    nuc: String,
    nuc_authority: String,
    judicial_case_number: String,
    judicial_authority: String,
    offenses: Vec<String>,
    general_information: Option<String>,
    complementary_identifiers: Option<String>,
}

impl ProfileData {
    fn validate(self) -> Result<PenalCaseProfile, ApplicationError> {
        Ok(PenalCaseProfile::new(
            &self.nuc,
            &self.nuc_authority,
            &self.judicial_case_number,
            &self.judicial_authority,
            &self.offenses.iter().map(String::as_str).collect::<Vec<_>>(),
            self.general_information.as_deref(),
            self.complementary_identifiers.as_deref(),
        )?)
    }
}

impl From<&PenalCaseProfile> for ProfileData {
    fn from(profile: &PenalCaseProfile) -> Self {
        Self {
            nuc: profile.nuc().into(),
            nuc_authority: profile.nuc_authority().into(),
            judicial_case_number: profile.judicial_case_number().into(),
            judicial_authority: profile.judicial_authority().into(),
            offenses: profile.offenses().to_vec(),
            general_information: profile.general_information().map(str::to_owned),
            complementary_identifiers: profile.complementary_identifiers().map(str::to_owned),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Creation {
    title: String,
    reference: String,
    profile: ProfileData,
}
impl Creation {
    pub fn validate(self) -> Result<PenalCaseCreation, ApplicationError> {
        Ok(PenalCaseCreation::new(
            CaseMetadata::new(&self.title, &self.reference)?,
            self.profile.validate()?,
        ))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Replacement {
    expected_revision: u32,
    title: String,
    reference: String,
    profile: Option<ProfileData>,
}
impl Replacement {
    pub fn validate(
        self,
    ) -> Result<(CaseRevisionExpectation, CaseEditableValues), ApplicationError> {
        Ok((
            CaseRevisionExpectation::new(self.expected_revision),
            CaseEditableValues::new(
                CaseMetadata::new(&self.title, &self.reference)?,
                self.profile.map(ProfileData::validate).transpose()?,
            ),
        ))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    Active,
    Closed,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StatusChange {
    expected_revision: u32,
    administrative_status: Status,
}
impl StatusChange {
    pub fn values(self) -> (CaseRevisionExpectation, CaseAdministrativeStatus) {
        let status = match self.administrative_status {
            Status::Active => CaseAdministrativeStatus::Active,
            Status::Closed => CaseAdministrativeStatus::Closed,
        };
        (CaseRevisionExpectation::new(self.expected_revision), status)
    }
}
