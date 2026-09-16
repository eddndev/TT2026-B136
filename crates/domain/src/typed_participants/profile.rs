use super::{
    Declared, ParticipantEvidenceLocator, ParticipantReason, ParticipantText, ProfessionalLicense,
    SubjectKind,
};
use crate::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustodyState {
    AtLiberty,
    Detained,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefenseMode {
    Private,
    Public,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourtComposition {
    Single,
    Collegiate,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclaredContact {
    Unknown(ParticipantReason),
    NoContactRecorded(ParticipantReason),
    Documented(ParticipantEvidenceLocator),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclaredProtection {
    Unknown(ParticipantReason),
    NoneDeclared(ParticipantReason),
    Documented(ParticipantEvidenceLocator),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticipantKind {
    Defendant,
    Victim,
    DefenseCounsel,
    Prosecutor,
    VictimCounsel,
    ControlJudge,
    TrialCourt,
    Expert,
    Police,
    PrecautionarySupervisor,
    Other,
}
impl ParticipantKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Defendant => "defendant",
            Self::Victim => "victim",
            Self::DefenseCounsel => "defense_counsel",
            Self::Prosecutor => "prosecutor",
            Self::VictimCounsel => "victim_counsel",
            Self::ControlJudge => "control_judge",
            Self::TrialCourt => "trial_court",
            Self::Expert => "expert",
            Self::Police => "police",
            Self::PrecautionarySupervisor => "precautionary_supervisor",
            Self::Other => "other",
        }
    }
    pub const fn tag(self) -> u8 {
        match self {
            Self::Defendant => 0,
            Self::Victim => 1,
            Self::DefenseCounsel => 2,
            Self::Prosecutor => 3,
            Self::VictimCounsel => 4,
            Self::ControlJudge => 5,
            Self::TrialCourt => 6,
            Self::Expert => 7,
            Self::Police => 8,
            Self::PrecautionarySupervisor => 9,
            Self::Other => 10,
        }
    }
    pub const fn accepts_subject(self, kind: SubjectKind) -> bool {
        match self {
            Self::Other => true,
            Self::TrialCourt => matches!(kind, SubjectKind::InstitutionalBody),
            _ => matches!(kind, SubjectKind::NaturalPerson),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParticipantProfile {
    Defendant(DefendantProfile),
    Victim(VictimProfile),
    DefenseCounsel(DefenseCounselProfile),
    Prosecutor(ProsecutorProfile),
    VictimCounsel(VictimCounselProfile),
    ControlJudge(ControlJudgeProfile),
    TrialCourt(TrialCourtProfile),
    Expert(ExpertProfile),
    Police(PoliceProfile),
    PrecautionarySupervisor(PrecautionarySupervisorProfile),
    Other(OtherParticipantProfile),
}
impl ParticipantProfile {
    pub const fn kind(&self) -> ParticipantKind {
        match self {
            Self::Defendant(_) => ParticipantKind::Defendant,
            Self::Victim(_) => ParticipantKind::Victim,
            Self::DefenseCounsel(_) => ParticipantKind::DefenseCounsel,
            Self::Prosecutor(_) => ParticipantKind::Prosecutor,
            Self::VictimCounsel(_) => ParticipantKind::VictimCounsel,
            Self::ControlJudge(_) => ParticipantKind::ControlJudge,
            Self::TrialCourt(_) => ParticipantKind::TrialCourt,
            Self::Expert(_) => ParticipantKind::Expert,
            Self::Police(_) => ParticipantKind::Police,
            Self::PrecautionarySupervisor(_) => ParticipantKind::PrecautionarySupervisor,
            Self::Other(_) => ParticipantKind::Other,
        }
    }
    pub const fn requires_credential(&self) -> bool {
        matches!(self, Self::DefenseCounsel(_) | Self::ControlJudge(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefendantProfile {
    custody: Declared<CustodyState>,
}
impl DefendantProfile {
    pub fn new(custody: Declared<CustodyState>) -> Self {
        Self { custody }
    }
    pub fn custody(&self) -> &Declared<CustodyState> {
        &self.custody
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VictimProfile {
    contact: DeclaredContact,
    protection: DeclaredProtection,
}
impl VictimProfile {
    pub fn new(contact: DeclaredContact, protection: DeclaredProtection) -> Self {
        Self {
            contact,
            protection,
        }
    }
    pub fn contact(&self) -> &DeclaredContact {
        &self.contact
    }
    pub fn protection(&self) -> &DeclaredProtection {
        &self.protection
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefenseCounselProfile {
    license: ProfessionalLicense,
    mode: DefenseMode,
}
impl DefenseCounselProfile {
    pub fn new(license: ProfessionalLicense, mode: DefenseMode) -> Self {
        Self { license, mode }
    }
    pub fn license(&self) -> &ProfessionalLicense {
        &self.license
    }
    pub fn mode(&self) -> &DefenseMode {
        &self.mode
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProsecutorProfile {
    office_identifier: Declared<ParticipantText<80>>,
    unit: Declared<ParticipantText<200>>,
    license: Declared<ProfessionalLicense>,
}
impl ProsecutorProfile {
    pub fn new(
        office_identifier: Declared<ParticipantText<80>>,
        unit: Declared<ParticipantText<200>>,
        license: Declared<ProfessionalLicense>,
    ) -> Self {
        Self {
            office_identifier,
            unit,
            license,
        }
    }
    pub fn office_identifier(&self) -> &Declared<ParticipantText<80>> {
        &self.office_identifier
    }
    pub fn unit(&self) -> &Declared<ParticipantText<200>> {
        &self.unit
    }
    pub fn license(&self) -> &Declared<ProfessionalLicense> {
        &self.license
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VictimCounselProfile {
    institution: ParticipantText<200>,
    license: Declared<ProfessionalLicense>,
}
impl VictimCounselProfile {
    pub fn new(institution: ParticipantText<200>, license: Declared<ProfessionalLicense>) -> Self {
        Self {
            institution,
            license,
        }
    }
    pub fn institution(&self) -> &ParticipantText<200> {
        &self.institution
    }
    pub fn license(&self) -> &Declared<ProfessionalLicense> {
        &self.license
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlJudgeProfile {
    court: ParticipantText<200>,
}
impl ControlJudgeProfile {
    pub fn new(court: &str) -> Result<Self, DomainError> {
        Ok(Self {
            court: ParticipantText::new(court)?,
        })
    }
    pub fn court(&self) -> &str {
        self.court.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrialCourtProfile {
    judicial_district: ParticipantText<200>,
    composition: CourtComposition,
}
impl TrialCourtProfile {
    pub fn new(judicial_district: ParticipantText<200>, composition: CourtComposition) -> Self {
        Self {
            judicial_district,
            composition,
        }
    }
    pub fn judicial_district(&self) -> &ParticipantText<200> {
        &self.judicial_district
    }
    pub fn composition(&self) -> &CourtComposition {
        &self.composition
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpertProfile {
    specialty: Declared<ParticipantText<200>>,
    license: Declared<ProfessionalLicense>,
}
impl ExpertProfile {
    pub fn new(
        specialty: Declared<ParticipantText<200>>,
        license: Declared<ProfessionalLicense>,
    ) -> Self {
        Self { specialty, license }
    }
    pub fn specialty(&self) -> &Declared<ParticipantText<200>> {
        &self.specialty
    }
    pub fn license(&self) -> &Declared<ProfessionalLicense> {
        &self.license
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoliceProfile {
    agency: Declared<ParticipantText<200>>,
    unit: Declared<ParticipantText<200>>,
}
impl PoliceProfile {
    pub fn new(
        agency: Declared<ParticipantText<200>>,
        unit: Declared<ParticipantText<200>>,
    ) -> Self {
        Self { agency, unit }
    }
    pub fn agency(&self) -> &Declared<ParticipantText<200>> {
        &self.agency
    }
    pub fn unit(&self) -> &Declared<ParticipantText<200>> {
        &self.unit
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecautionarySupervisorProfile {
    authority: Declared<ParticipantText<200>>,
    unit: Declared<ParticipantText<200>>,
}
impl PrecautionarySupervisorProfile {
    pub fn new(
        authority: Declared<ParticipantText<200>>,
        unit: Declared<ParticipantText<200>>,
    ) -> Self {
        Self { authority, unit }
    }
    pub fn authority(&self) -> &Declared<ParticipantText<200>> {
        &self.authority
    }
    pub fn unit(&self) -> &Declared<ParticipantText<200>> {
        &self.unit
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OtherParticipantProfile {
    label: ParticipantText<80>,
    description: Declared<ParticipantText<200>>,
}
impl OtherParticipantProfile {
    pub fn new(label: ParticipantText<80>, description: Declared<ParticipantText<200>>) -> Self {
        Self { label, description }
    }
    pub fn label(&self) -> &ParticipantText<80> {
        &self.label
    }
    pub fn description(&self) -> &Declared<ParticipantText<200>> {
        &self.description
    }
}
