use super::*;

impl SubjectValues {
    /// SUBJ1 encodes declared values and exact evidence, without actor or time.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = b"SUBJ1".to_vec();
        match self {
            Self::NaturalPerson {
                name,
                curp,
                identity_support,
            } => {
                out.push(0);
                match name {
                    RepresentedName::Known(value) => {
                        out.push(0);
                        text(&mut out, value.as_str());
                    }
                    RepresentedName::Unidentified { label, reason } => {
                        out.push(1);
                        text(&mut out, label.as_str());
                        text(&mut out, reason.as_str());
                    }
                }
                declared(&mut out, curp, |out, value| text(out, value.as_str()));
                locator(&mut out, identity_support);
            }
            Self::InstitutionalBody {
                name,
                institutional_identifier,
                identity_support,
            } => {
                out.push(1);
                text(&mut out, name.as_str());
                declared(&mut out, institutional_identifier, |out, value| {
                    text(out, value.as_str())
                });
                locator(&mut out, identity_support);
            }
        }
        out
    }
}
impl TypedParticipantValues {
    /// PART2 pins the represented identity revision; credentials are separate.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = b"PART2".to_vec();
        let subject = self.subject();
        out.extend_from_slice(subject.id.as_uuid().as_bytes());
        out.extend_from_slice(&subject.revision.get().to_be_bytes());
        out.extend_from_slice(subject.values_digest.as_bytes());
        out.push(match self.directory_status() {
            DirectoryStatus::Active => 0,
            DirectoryStatus::Archived => 1,
        });
        for value in [self.role().organization(), self.role().legal_status()] {
            match value {
                None => out.push(0),
                Some(value) => {
                    out.push(1);
                    text(&mut out, value);
                }
            }
        }
        out.push(self.kind().tag());
        profile(&mut out, self.role().profile());
        locator(&mut out, self.role().role_support());
        out
    }
}
fn profile(out: &mut Vec<u8>, value: &ParticipantProfile) {
    match value {
        ParticipantProfile::Defendant(v) => declared(out, v.custody(), |o, v| {
            o.push(match v {
                CustodyState::AtLiberty => 0,
                CustodyState::Detained => 1,
            })
        }),
        ParticipantProfile::Victim(v) => {
            match v.contact() {
                DeclaredContact::Unknown(r) => {
                    out.push(0);
                    text(out, r.as_str());
                }
                DeclaredContact::NoContactRecorded(r) => {
                    out.push(1);
                    text(out, r.as_str());
                }
                DeclaredContact::Documented(v) => {
                    out.push(2);
                    locator(out, v);
                }
            }
            match v.protection() {
                DeclaredProtection::Unknown(r) => {
                    out.push(0);
                    text(out, r.as_str());
                }
                DeclaredProtection::NoneDeclared(r) => {
                    out.push(1);
                    text(out, r.as_str());
                }
                DeclaredProtection::Documented(v) => {
                    out.push(2);
                    locator(out, v);
                }
            }
        }
        ParticipantProfile::DefenseCounsel(v) => {
            license(out, v.license());
            out.push(match v.mode() {
                DefenseMode::Private => 0,
                DefenseMode::Public => 1,
            });
        }
        ParticipantProfile::Prosecutor(v) => {
            declared_text(out, v.office_identifier());
            declared_text(out, v.unit());
            declared(out, v.license(), license);
        }
        ParticipantProfile::VictimCounsel(v) => {
            text(out, v.institution().as_str());
            declared(out, v.license(), license);
        }
        ParticipantProfile::ControlJudge(v) => text(out, v.court()),
        ParticipantProfile::TrialCourt(v) => {
            text(out, v.judicial_district().as_str());
            out.push(match v.composition() {
                CourtComposition::Single => 0,
                CourtComposition::Collegiate => 1,
            });
        }
        ParticipantProfile::Expert(v) => {
            declared_text(out, v.specialty());
            declared(out, v.license(), license);
        }
        ParticipantProfile::Police(v) => {
            declared_text(out, v.agency());
            declared_text(out, v.unit());
        }
        ParticipantProfile::PrecautionarySupervisor(v) => {
            declared_text(out, v.authority());
            declared_text(out, v.unit());
        }
        ParticipantProfile::Other(v) => {
            text(out, v.label().as_str());
            declared_text(out, v.description());
        }
    }
}
fn text(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u32).to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}
fn declared<T>(out: &mut Vec<u8>, value: &Declared<T>, write: impl FnOnce(&mut Vec<u8>, &T)) {
    match value {
        Declared::Known(value) => {
            out.push(0);
            write(out, value);
        }
        Declared::Unknown(reason) => {
            out.push(1);
            text(out, reason.as_str());
        }
    }
}
fn declared_text<const MAX: usize>(out: &mut Vec<u8>, value: &Declared<ParticipantText<MAX>>) {
    declared(out, value, |out, value| text(out, value.as_str()));
}
fn license(out: &mut Vec<u8>, value: &ProfessionalLicense) {
    text(out, value.number());
    text(out, value.issuer());
}
fn locator(out: &mut Vec<u8>, value: &ParticipantEvidenceLocator) {
    out.extend_from_slice(value.reference().id.as_uuid().as_bytes());
    out.extend_from_slice(&value.reference().version.get().to_be_bytes());
    out.extend_from_slice(value.digest().as_bytes());
    text(out, value.locator());
}
