use super::values::{text, Declared, License, Locator};
use application::{typed_participants as m, ApplicationError as Error};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Custody {
    AtLiberty,
    Detained,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Defense {
    Private,
    Public,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Composition {
    Single,
    Collegiate,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Contact {
    Unknown { reason: String },
    NotRecorded { reason: String },
    Documented { support: Locator },
}
#[derive(Deserialize, Serialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Protection {
    Unknown { reason: String },
    NoneDeclared { reason: String },
    Documented { support: Locator },
}
impl Contact {
    fn validate(self) -> Result<m::DeclaredContact, Error> {
        Ok(match self {
            Self::Unknown { reason } => {
                m::DeclaredContact::Unknown(m::ParticipantReason::new(&reason)?)
            }
            Self::NotRecorded { reason } => {
                m::DeclaredContact::NoContactRecorded(m::ParticipantReason::new(&reason)?)
            }
            Self::Documented { support } => m::DeclaredContact::Documented(support.validate()?),
        })
    }
}
impl From<&m::DeclaredContact> for Contact {
    fn from(v: &m::DeclaredContact) -> Self {
        match v {
            m::DeclaredContact::Unknown(reason) => Self::Unknown {
                reason: reason.as_str().into(),
            },
            m::DeclaredContact::NoContactRecorded(reason) => Self::NotRecorded {
                reason: reason.as_str().into(),
            },
            m::DeclaredContact::Documented(support) => Self::Documented {
                support: support.into(),
            },
        }
    }
}
impl Protection {
    fn validate(self) -> Result<m::DeclaredProtection, Error> {
        Ok(match self {
            Self::Unknown { reason } => {
                m::DeclaredProtection::Unknown(m::ParticipantReason::new(&reason)?)
            }
            Self::NoneDeclared { reason } => {
                m::DeclaredProtection::NoneDeclared(m::ParticipantReason::new(&reason)?)
            }
            Self::Documented { support } => m::DeclaredProtection::Documented(support.validate()?),
        })
    }
}
impl From<&m::DeclaredProtection> for Protection {
    fn from(v: &m::DeclaredProtection) -> Self {
        match v {
            m::DeclaredProtection::Unknown(reason) => Self::Unknown {
                reason: reason.as_str().into(),
            },
            m::DeclaredProtection::NoneDeclared(reason) => Self::NoneDeclared {
                reason: reason.as_str().into(),
            },
            m::DeclaredProtection::Documented(support) => Self::Documented {
                support: support.into(),
            },
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Profile {
    Defendant {
        custody: Declared<Custody>,
    },
    Victim {
        contact: Contact,
        protection: Protection,
    },
    DefenseCounsel {
        license: License,
        mode: Defense,
    },
    Prosecutor {
        office_identifier: Declared<String>,
        unit: Declared<String>,
        license: Declared<License>,
    },
    VictimCounsel {
        institution: String,
        license: Declared<License>,
    },
    ControlJudge {
        court: String,
    },
    TrialCourt {
        judicial_district: String,
        composition: Composition,
    },
    Expert {
        specialty: Declared<String>,
        license: Declared<License>,
    },
    Police {
        agency: Declared<String>,
        unit: Declared<String>,
    },
    PrecautionarySupervisor {
        authority: Declared<String>,
        unit: Declared<String>,
    },
    Other {
        label: String,
        description: Declared<String>,
    },
}
impl Profile {
    pub fn validate(self) -> Result<m::ParticipantProfile, Error> {
        use m::ParticipantProfile as P;
        Ok(match self {
            Self::Defendant { custody } => {
                P::Defendant(m::DefendantProfile::new(custody.validate(|v| {
                    Ok(match v {
                        Custody::AtLiberty => m::CustodyState::AtLiberty,
                        Custody::Detained => m::CustodyState::Detained,
                    })
                })?))
            }
            Self::Victim {
                contact,
                protection,
            } => P::Victim(m::VictimProfile::new(
                contact.validate()?,
                protection.validate()?,
            )),
            Self::DefenseCounsel { license, mode } => {
                P::DefenseCounsel(m::DefenseCounselProfile::new(
                    license.validate()?,
                    match mode {
                        Defense::Private => m::DefenseMode::Private,
                        Defense::Public => m::DefenseMode::Public,
                    },
                ))
            }
            Self::Prosecutor {
                office_identifier,
                unit,
                license,
            } => P::Prosecutor(m::ProsecutorProfile::new(
                office_identifier.validate(text)?,
                unit.validate(text)?,
                license.validate(License::validate)?,
            )),
            Self::VictimCounsel {
                institution,
                license,
            } => P::VictimCounsel(m::VictimCounselProfile::new(
                text(institution)?,
                license.validate(License::validate)?,
            )),
            Self::ControlJudge { court } => P::ControlJudge(m::ControlJudgeProfile::new(&court)?),
            Self::TrialCourt {
                judicial_district,
                composition,
            } => P::TrialCourt(m::TrialCourtProfile::new(
                text(judicial_district)?,
                match composition {
                    Composition::Single => m::CourtComposition::Single,
                    Composition::Collegiate => m::CourtComposition::Collegiate,
                },
            )),
            Self::Expert { specialty, license } => P::Expert(m::ExpertProfile::new(
                specialty.validate(text)?,
                license.validate(License::validate)?,
            )),
            Self::Police { agency, unit } => P::Police(m::PoliceProfile::new(
                agency.validate(text)?,
                unit.validate(text)?,
            )),
            Self::PrecautionarySupervisor { authority, unit } => {
                P::PrecautionarySupervisor(m::PrecautionarySupervisorProfile::new(
                    authority.validate(text)?,
                    unit.validate(text)?,
                ))
            }
            Self::Other { label, description } => P::Other(m::OtherParticipantProfile::new(
                text(label)?,
                description.validate(text)?,
            )),
        })
    }
}
impl From<&m::ParticipantProfile> for Profile {
    fn from(v: &m::ParticipantProfile) -> Self {
        use m::ParticipantProfile as P;
        match v {
            P::Defendant(v) => Self::Defendant {
                custody: Declared::from_value(v.custody(), |v| match v {
                    m::CustodyState::AtLiberty => Custody::AtLiberty,
                    m::CustodyState::Detained => Custody::Detained,
                }),
            },
            P::Victim(v) => Self::Victim {
                contact: v.contact().into(),
                protection: v.protection().into(),
            },
            P::DefenseCounsel(v) => Self::DefenseCounsel {
                license: v.license().into(),
                mode: match v.mode() {
                    m::DefenseMode::Private => Defense::Private,
                    m::DefenseMode::Public => Defense::Public,
                },
            },
            P::Prosecutor(v) => Self::Prosecutor {
                office_identifier: declared_text(v.office_identifier()),
                unit: declared_text(v.unit()),
                license: declared_license(v.license()),
            },
            P::VictimCounsel(v) => Self::VictimCounsel {
                institution: v.institution().as_str().into(),
                license: declared_license(v.license()),
            },
            P::ControlJudge(v) => Self::ControlJudge {
                court: v.court().into(),
            },
            P::TrialCourt(v) => Self::TrialCourt {
                judicial_district: v.judicial_district().as_str().into(),
                composition: match v.composition() {
                    m::CourtComposition::Single => Composition::Single,
                    m::CourtComposition::Collegiate => Composition::Collegiate,
                },
            },
            P::Expert(v) => Self::Expert {
                specialty: declared_text(v.specialty()),
                license: declared_license(v.license()),
            },
            P::Police(v) => Self::Police {
                agency: declared_text(v.agency()),
                unit: declared_text(v.unit()),
            },
            P::PrecautionarySupervisor(v) => Self::PrecautionarySupervisor {
                authority: declared_text(v.authority()),
                unit: declared_text(v.unit()),
            },
            P::Other(v) => Self::Other {
                label: v.label().as_str().into(),
                description: declared_text(v.description()),
            },
        }
    }
}
fn declared_text<const MAX: usize>(v: &m::Declared<m::ParticipantText<MAX>>) -> Declared<String> {
    Declared::from_value(v, |v| v.as_str().into())
}
fn declared_license(v: &m::Declared<m::ProfessionalLicense>) -> Declared<License> {
    Declared::from_value(v, |v| v.into())
}
