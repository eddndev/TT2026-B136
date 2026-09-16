use domain::{
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest},
    typed_participants::*,
};
use serde_json::Value;

fn text(v: &Value) -> &str {
    v.as_str().unwrap()
}
fn ptext<const N: usize>(v: &Value) -> ParticipantText<N> {
    ParticipantText::new(text(v)).unwrap()
}
fn reason(v: &Value) -> ParticipantReason {
    ParticipantReason::new(text(v)).unwrap()
}
fn declared<T>(v: &Value, f: impl FnOnce(&Value) -> T) -> Declared<T> {
    if v.get("known").is_some() {
        Declared::Known(f(&v["known"]))
    } else {
        Declared::Unknown(reason(&v["unknown"]))
    }
}
fn license(v: &Value) -> ProfessionalLicense {
    ProfessionalLicense::new(text(&v["number"]), text(&v["issuer"])).unwrap()
}
fn support(v: &Value) -> ParticipantEvidenceLocator {
    ParticipantEvidenceLocator::new(
        DocumentVersionRef {
            id: DocumentId::from_uuid(Uuid::parse_str(text(&v["document_id"])).unwrap()),
            version: DocumentVersion::new(v["version"].as_u64().unwrap() as u32).unwrap(),
        },
        Sha256Digest::from_hex(text(&v["digest"])).unwrap(),
        text(&v["locator"]),
    )
    .unwrap()
}
pub fn subject(v: &Value) -> SubjectValues {
    if text(&v["kind"]) == "natural_person" {
        let name = &v["name"];
        let name = if name.get("known").is_some() {
            RepresentedName::Known(ptext(&name["known"]))
        } else {
            RepresentedName::Unidentified {
                label: ptext(&name["label"]),
                reason: reason(&name["reason"]),
            }
        };
        SubjectValues::natural_person(
            name,
            declared(&v["curp"], |v| Curp::new(text(v)).unwrap()),
            support(&v["identity_support"]),
        )
    } else {
        SubjectValues::institutional_body(
            ptext(&v["name"]),
            declared(&v["institutional_identifier"], ptext),
            support(&v["identity_support"]),
        )
    }
}
fn contact(v: &Value) -> DeclaredContact {
    if v.get("documented").is_some() {
        DeclaredContact::Documented(support(&v["documented"]))
    } else if v.get("unknown").is_some() {
        DeclaredContact::Unknown(reason(&v["unknown"]))
    } else {
        DeclaredContact::NoContactRecorded(reason(&v["none"]))
    }
}
fn protection(v: &Value) -> DeclaredProtection {
    if v.get("documented").is_some() {
        DeclaredProtection::Documented(support(&v["documented"]))
    } else if v.get("unknown").is_some() {
        DeclaredProtection::Unknown(reason(&v["unknown"]))
    } else {
        DeclaredProtection::NoneDeclared(reason(&v["none"]))
    }
}
fn profile(v: &Value) -> ParticipantProfile {
    match text(&v["kind"]) {
        "defendant" => {
            ParticipantProfile::Defendant(DefendantProfile::new(declared(&v["custody"], |v| {
                if text(v) == "at_liberty" {
                    CustodyState::AtLiberty
                } else {
                    CustodyState::Detained
                }
            })))
        }
        "victim" => ParticipantProfile::Victim(VictimProfile::new(
            contact(&v["contact"]),
            protection(&v["protection"]),
        )),
        "defense_counsel" => ParticipantProfile::DefenseCounsel(DefenseCounselProfile::new(
            license(&v["license"]),
            if text(&v["mode"]) == "private" {
                DefenseMode::Private
            } else {
                DefenseMode::Public
            },
        )),
        "prosecutor" => ParticipantProfile::Prosecutor(ProsecutorProfile::new(
            declared(&v["office_identifier"], ptext),
            declared(&v["unit"], ptext),
            declared(&v["license"], license),
        )),
        "victim_counsel" => ParticipantProfile::VictimCounsel(VictimCounselProfile::new(
            ptext(&v["institution"]),
            declared(&v["license"], license),
        )),
        "control_judge" => {
            ParticipantProfile::ControlJudge(ControlJudgeProfile::new(text(&v["court"])).unwrap())
        }
        "trial_court" => ParticipantProfile::TrialCourt(TrialCourtProfile::new(
            ptext(&v["judicial_district"]),
            if text(&v["composition"]) == "single" {
                CourtComposition::Single
            } else {
                CourtComposition::Collegiate
            },
        )),
        "expert" => ParticipantProfile::Expert(ExpertProfile::new(
            declared(&v["specialty"], ptext),
            declared(&v["license"], license),
        )),
        "police" => ParticipantProfile::Police(PoliceProfile::new(
            declared(&v["agency"], ptext),
            declared(&v["unit"], ptext),
        )),
        "precautionary_supervisor" => {
            ParticipantProfile::PrecautionarySupervisor(PrecautionarySupervisorProfile::new(
                declared(&v["authority"], ptext),
                declared(&v["unit"], ptext),
            ))
        }
        "other" => ParticipantProfile::Other(OtherParticipantProfile::new(
            ptext(&v["label"]),
            declared(&v["description"], ptext),
        )),
        other => panic!("unexpected vector kind {other}"),
    }
}
pub fn participant(v: &Value) -> TypedParticipantValues {
    let s = &v["subject"];
    let subject = SubjectRevisionRef {
        id: CaseSubjectId::from_uuid(Uuid::parse_str(text(&s["id"])).unwrap()),
        revision: SubjectRevision::new(s["revision"].as_u64().unwrap() as u32).unwrap(),
        values_digest: Sha256Digest::from_hex(text(&s["digest"])).unwrap(),
    };
    TypedParticipantValues::new(
        subject,
        if text(&v["directory_status"]) == "active" {
            DirectoryStatus::Active
        } else {
            DirectoryStatus::Archived
        },
        ParticipantRoleValues::new(
            v["organization"].as_str(),
            v["legal_status"].as_str(),
            profile(&v["profile"]),
            support(&v["role_support"]),
        )
        .unwrap(),
    )
}
