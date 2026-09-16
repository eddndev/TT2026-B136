use domain::typed_participants::*;
use serde_json::Value;

use super::{inconsistent, primitives::*, Result};

pub(super) fn parse(value: &Value) -> Result<ParticipantProfile> {
    Ok(match string(&value["kind"])? {
        "defendant" => {
            fields(value, &["kind", "custody"])?;
            ParticipantProfile::Defendant(DefendantProfile::new(declared(
                &value["custody"],
                |v| match string(v)? {
                    "at_liberty" => Ok(CustodyState::AtLiberty),
                    "detained" => Ok(CustodyState::Detained),
                    _ => Err(inconsistent()),
                },
            )?))
        }
        "victim" => {
            fields(value, &["kind", "contact", "protection"])?;
            ParticipantProfile::Victim(VictimProfile::new(
                contact(&value["contact"])?,
                protection(&value["protection"])?,
            ))
        }
        "defense_counsel" => {
            fields(value, &["kind", "license", "mode"])?;
            let mode = match string(&value["mode"])? {
                "private" => DefenseMode::Private,
                "public" => DefenseMode::Public,
                _ => return Err(inconsistent()),
            };
            ParticipantProfile::DefenseCounsel(DefenseCounselProfile::new(
                license(&value["license"])?,
                mode,
            ))
        }
        "prosecutor" => {
            fields(value, &["kind", "office_identifier", "unit", "license"])?;
            ParticipantProfile::Prosecutor(ProsecutorProfile::new(
                declared(&value["office_identifier"], text)?,
                declared(&value["unit"], text)?,
                declared(&value["license"], license)?,
            ))
        }
        "victim_counsel" => {
            fields(value, &["kind", "institution", "license"])?;
            ParticipantProfile::VictimCounsel(VictimCounselProfile::new(
                text(&value["institution"])?,
                declared(&value["license"], license)?,
            ))
        }
        "control_judge" => {
            fields(value, &["kind", "court"])?;
            ParticipantProfile::ControlJudge(
                ControlJudgeProfile::new(text::<200>(&value["court"])?.as_str())
                    .map_err(|_| inconsistent())?,
            )
        }
        "trial_court" => {
            fields(value, &["kind", "judicial_district", "composition"])?;
            let composition = match string(&value["composition"])? {
                "single" => CourtComposition::Single,
                "collegiate" => CourtComposition::Collegiate,
                _ => return Err(inconsistent()),
            };
            ParticipantProfile::TrialCourt(TrialCourtProfile::new(
                text(&value["judicial_district"])?,
                composition,
            ))
        }
        "expert" => {
            fields(value, &["kind", "specialty", "license"])?;
            ParticipantProfile::Expert(ExpertProfile::new(
                declared(&value["specialty"], text)?,
                declared(&value["license"], license)?,
            ))
        }
        "police" => {
            fields(value, &["kind", "agency", "unit"])?;
            ParticipantProfile::Police(PoliceProfile::new(
                declared(&value["agency"], text)?,
                declared(&value["unit"], text)?,
            ))
        }
        "precautionary_supervisor" => {
            fields(value, &["kind", "authority", "unit"])?;
            ParticipantProfile::PrecautionarySupervisor(PrecautionarySupervisorProfile::new(
                declared(&value["authority"], text)?,
                declared(&value["unit"], text)?,
            ))
        }
        "other" => {
            fields(value, &["kind", "label", "description"])?;
            ParticipantProfile::Other(OtherParticipantProfile::new(
                text(&value["label"])?,
                declared(&value["description"], text)?,
            ))
        }
        _ => return Err(inconsistent()),
    })
}

fn contact(value: &Value) -> Result<DeclaredContact> {
    if value.get("documented").is_some() {
        fields(value, &["documented"])?;
        Ok(DeclaredContact::Documented(support(&value["documented"])?))
    } else if value.get("unknown").is_some() {
        fields(value, &["unknown"])?;
        Ok(DeclaredContact::Unknown(reason(&value["unknown"])?))
    } else {
        fields(value, &["none"])?;
        Ok(DeclaredContact::NoContactRecorded(reason(&value["none"])?))
    }
}

fn protection(value: &Value) -> Result<DeclaredProtection> {
    if value.get("documented").is_some() {
        fields(value, &["documented"])?;
        Ok(DeclaredProtection::Documented(support(
            &value["documented"],
        )?))
    } else if value.get("unknown").is_some() {
        fields(value, &["unknown"])?;
        Ok(DeclaredProtection::Unknown(reason(&value["unknown"])?))
    } else {
        fields(value, &["none"])?;
        Ok(DeclaredProtection::NoneDeclared(reason(&value["none"])?))
    }
}
