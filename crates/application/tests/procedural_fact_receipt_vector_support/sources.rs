use super::{byte, declared_time, digest, number, offset, string, text, uuid};
use application::{
    case_stages::StageSupportSnapshot, documents::*, procedural_facts::*,
    typed_participants::ParticipantOverview,
};
use domain::{
    cases::CaseId,
    crypto::{DocumentId, DocumentVersion},
    hearing_results::*,
    hearings::HearingId,
    judicial_calendars::CivilDate,
    participants::*,
    typed_participants::{CaseSubjectId, ParticipantKind, SubjectRevision, SubjectRevisionRef},
};
use serde_json::Value;
use time::Time;

fn resolution(value: &Value) -> (FactResolutionSourceSnapshot, FactResolutionView) {
    let reference = FactResolutionRef {
        id: ResolutionId::from_uuid(uuid(&value["id"])),
        revision: FactRevision::new(number(&value["revision"])).unwrap(),
    };
    let class = &value["class"];
    let class = if let Some(reason) = class.get("unknown") {
        FactDeclaration::Unknown(text(string(reason)))
    } else {
        let known = &class["known"];
        FactDeclaration::Known(match string(&known["kind"]) {
            "order" => ResolutionClass::Order,
            "judgment" => ResolutionClass::Judgment,
            "other" => ResolutionClass::Other(FactLabel::new(string(&known["label"])).unwrap()),
            _ => panic!("unsupported fixture class"),
        })
    };
    let issuer = &value["issuer"];
    let issuer = if let Some(reason) = issuer.get("unknown") {
        FactDeclaration::Unknown(text(string(reason)))
    } else {
        FactDeclaration::Known(FactLabel::new(string(&issuer["known"])).unwrap())
    };
    (
        FactResolutionSourceSnapshot {
            case_id: CaseId::from_uuid(uuid(&value["case"])),
            reference,
            values_digest: digest(&value["values_digest"]),
            submission_digest: digest(&value["submission_digest"]),
            status: match string(&value["status"]) {
                "recorded" => FactStatus::Recorded,
                "withdrawn" => FactStatus::Withdrawn,
                _ => panic!("unsupported fixture status"),
            },
        },
        FactResolutionView {
            reference,
            class,
            issuer,
            issued_at: declared_time(&value["issued_at"]),
            summary: text(string(&value["summary"])),
        },
    )
}
fn participant(value: &Value) -> (FactParticipantSnapshot, ParticipantOverview) {
    let reference = FactParticipantRef {
        id: ParticipantId::from_uuid(uuid(&value["id"])),
        revision: ParticipantRevision::new(number(&value["revision"])).unwrap(),
    };
    let subject = (!value["subject"].is_null()).then(|| {
        let value = &value["subject"];
        SubjectRevisionRef {
            id: CaseSubjectId::from_uuid(uuid(&value["id"])),
            revision: SubjectRevision::new(number(&value["revision"])).unwrap(),
            values_digest: digest(&value["values_digest"]),
        }
    });
    let kind = (!value["kind"].is_null()).then(|| match string(&value["kind"]) {
        "defendant" => ParticipantKind::Defendant,
        "victim" => ParticipantKind::Victim,
        "defense_counsel" => ParticipantKind::DefenseCounsel,
        "prosecutor" => ParticipantKind::Prosecutor,
        "victim_counsel" => ParticipantKind::VictimCounsel,
        "control_judge" => ParticipantKind::ControlJudge,
        "trial_court" => ParticipantKind::TrialCourt,
        "expert" => ParticipantKind::Expert,
        "police" => ParticipantKind::Police,
        "precautionary_supervisor" => ParticipantKind::PrecautionarySupervisor,
        "other" => ParticipantKind::Other,
        _ => panic!("unsupported fixture participant kind"),
    });
    let case_id = CaseId::from_uuid(uuid(&value["case"]));
    let status = match string(&value["status"]) {
        "active" => DirectoryStatus::Active,
        "archived" => DirectoryStatus::Archived,
        _ => panic!("unsupported fixture directory status"),
    };
    (
        FactParticipantSnapshot {
            case_id,
            reference,
            values_digest: digest(&value["values_digest"]),
            status,
            subject,
        },
        ParticipantOverview {
            case_id,
            id: reference.id,
            revision: reference.revision,
            display_name: string(&value["display_name"]).into(),
            procedural_role: string(&value["procedural_role"]).into(),
            organization: value["organization"].as_str().map(str::to_owned),
            directory_status: status,
            kind,
            subject,
        },
    )
}
fn hearing(value: &Value) -> (FactHearingSourceSnapshot, FactHearingView) {
    let reference = FactHearingRef {
        hearing_id: HearingId::from_uuid(uuid(&value["hearing_id"])),
        result_id: HearingResultId::from_uuid(uuid(&value["result_id"])),
        revision: HearingResultRevision::new(number(&value["revision"])).unwrap(),
        agreement_id: (!value["agreement_id"].is_null())
            .then(|| HearingResultAgreementId::from_uuid(uuid(&value["agreement_id"]))),
    };
    let temporal = &value["event_time"];
    let date = string(&temporal["date"])
        .parse::<CivilDate>()
        .unwrap()
        .date();
    let offset = offset(&temporal["offset_seconds"]);
    let event_time = match string(&temporal["precision"]) {
        "date" => DeclaredHearingResultTime::date(date, offset),
        "instant" => DeclaredHearingResultTime::instant(
            date.with_time(
                Time::from_hms(
                    byte(&temporal["hour"]),
                    byte(&temporal["minute"]),
                    byte(&temporal["second"]),
                )
                .unwrap(),
            )
            .assume_offset(offset),
        ),
        _ => panic!("unsupported fixture hearing time"),
    }
    .unwrap();
    (
        FactHearingSourceSnapshot {
            case_id: CaseId::from_uuid(uuid(&value["case"])),
            reference,
            values_digest: digest(&value["values_digest"]),
            submission_digest: digest(&value["submission_digest"]),
            status: match string(&value["status"]) {
                "recorded" => HearingResultStatus::Recorded,
                "withdrawn" => HearingResultStatus::Withdrawn,
                _ => panic!("unsupported fixture result status"),
            },
        },
        FactHearingView {
            reference,
            occurrence: match string(&value["occurrence"]) {
                "occurred" => HearingResultOccurrence::Occurred,
                "not_started" => HearingResultOccurrence::NotStarted,
                _ => panic!("unsupported fixture occurrence"),
            },
            event_time,
            summary: HearingResultText::new(string(&value["summary"])).unwrap(),
            agreement: reference.agreement_id.map(|id| {
                HearingResultAgreement::new(
                    id,
                    HearingResultText::new(string(&value["agreement_text"])).unwrap(),
                )
            }),
        },
    )
}
fn support(value: &Value) -> StageSupportSnapshot {
    assert_eq!(value["policy"], "pdf_docx_v1");
    StageSupportSnapshot {
        reference: DocumentVersionRef {
            id: DocumentId::from_uuid(uuid(&value["id"])),
            version: DocumentVersion::new(number(&value["version"])).unwrap(),
        },
        digest: digest(&value["digest"]),
        name: string(&value["name"]).into(),
        format: match string(&value["format"]) {
            "pdf" => StageDocumentFormat::Pdf,
            "docx" => StageDocumentFormat::Docx,
            _ => panic!("unsupported fixture format"),
        },
        policy: StageFormatPolicy::PdfDocxV1,
    }
}
pub fn sources(value: &Value) -> FactSources {
    let parent = (!value["resolution"].is_null()).then(|| resolution(&value["resolution"]));
    let (participants, participant_views) = value["participants"]
        .as_array()
        .unwrap()
        .iter()
        .map(participant)
        .unzip();
    let (hearing_results, hearing_views) = value["hearing_results"]
        .as_array()
        .unwrap()
        .iter()
        .map(hearing)
        .unzip();
    FactSources {
        resolved: FactResolvedSources {
            resolution: parent.as_ref().map(|pair| pair.0),
            participants,
            hearing_results,
        },
        views: FactSourceViews {
            resolution: parent.map(|pair| pair.1),
            participants: participant_views,
            hearing_results: hearing_views,
        },
        direct_supports: value["direct_supports"]
            .as_array()
            .unwrap()
            .iter()
            .map(support)
            .collect(),
    }
}
