use super::{helpers::*, inconsistent, temporal, Result};
use application::{
    case_stages::StageSupportSnapshot,
    documents::{StageDocumentFormat, StageFormatPolicy},
    procedural_facts::*,
    typed_participants::ParticipantOverview,
};
use domain::{
    cases::CaseId,
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef},
    hearing_results::{
        HearingResultAgreement, HearingResultAgreementId, HearingResultId, HearingResultRevision,
    },
    hearings::HearingId,
    participants::{ParticipantId, ParticipantRevision},
    typed_participants::{CaseSubjectId, ParticipantKind, SubjectRevision, SubjectRevisionRef},
};
use serde_json::Value;

fn subject(value: &Value) -> Result<SubjectRevisionRef> {
    fields(value, &["id", "revision", "values_digest"])?;
    Ok(SubjectRevisionRef {
        id: CaseSubjectId::from_uuid(uuid(&value["id"])?),
        revision: SubjectRevision::new(counter(&value["revision"])?).map_err(|_| inconsistent())?,
        values_digest: digest(&value["values_digest"])?,
    })
}
fn kind(value: &Value) -> Result<ParticipantKind> {
    Ok(match string(value)? {
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
        _ => return Err(inconsistent()),
    })
}
pub(super) fn participant(value: &Value) -> Result<(FactParticipantSnapshot, ParticipantOverview)> {
    fields(
        value,
        &[
            "case",
            "id",
            "revision",
            "values_digest",
            "status",
            "subject",
            "display_name",
            "procedural_role",
            "organization",
            "kind",
        ],
    )?;
    let case_id = CaseId::from_uuid(uuid(&value["case"])?);
    let reference = FactParticipantRef {
        id: ParticipantId::from_uuid(uuid(&value["id"])?),
        revision: ParticipantRevision::new(counter(&value["revision"])?)
            .map_err(|_| inconsistent())?,
    };
    let status = string(&value["status"])?
        .parse()
        .map_err(|_| inconsistent())?;
    let subject = optional(&value["subject"], subject)?;
    Ok((
        FactParticipantSnapshot {
            case_id,
            reference,
            values_digest: digest(&value["values_digest"])?,
            status,
            subject,
        },
        ParticipantOverview {
            case_id,
            id: reference.id,
            revision: reference.revision,
            display_name: owned(&value["display_name"], 800)?,
            procedural_role: owned(&value["procedural_role"], 320)?,
            organization: optional(&value["organization"], |v| owned(v, 800))?,
            directory_status: status,
            kind: optional(&value["kind"], kind)?,
            subject,
        },
    ))
}
pub(super) fn hearing(value: &Value) -> Result<(FactHearingSourceSnapshot, FactHearingView)> {
    fields(
        value,
        &[
            "case",
            "hearing_id",
            "result_id",
            "revision",
            "agreement_id",
            "values_digest",
            "submission_digest",
            "status",
            "occurrence",
            "event_time",
            "summary",
            "agreement_text",
        ],
    )?;
    let reference = FactHearingRef {
        hearing_id: HearingId::from_uuid(uuid(&value["hearing_id"])?),
        result_id: HearingResultId::from_uuid(uuid(&value["result_id"])?),
        revision: HearingResultRevision::new(counter(&value["revision"])?)
            .map_err(|_| inconsistent())?,
        agreement_id: optional(&value["agreement_id"], |v| {
            Ok(HearingResultAgreementId::from_uuid(uuid(v)?))
        })?,
    };
    let agreement = match (
        reference.agreement_id,
        optional(&value["agreement_text"], hearing_text)?,
    ) {
        (None, None) => None,
        (Some(id), Some(text)) => Some(HearingResultAgreement::new(id, text)),
        _ => return Err(inconsistent()),
    };
    Ok((
        FactHearingSourceSnapshot {
            case_id: CaseId::from_uuid(uuid(&value["case"])?),
            reference,
            values_digest: digest(&value["values_digest"])?,
            submission_digest: digest(&value["submission_digest"])?,
            status: string(&value["status"])?
                .parse()
                .map_err(|_| inconsistent())?,
        },
        FactHearingView {
            reference,
            occurrence: string(&value["occurrence"])?
                .parse()
                .map_err(|_| inconsistent())?,
            event_time: temporal::hearing_time(&value["event_time"])?,
            summary: hearing_text(&value["summary"])?,
            agreement,
        },
    ))
}
pub(super) fn support(value: &Value) -> Result<StageSupportSnapshot> {
    fields(
        value,
        &["id", "version", "digest", "name", "format", "policy"],
    )?;
    if string(&value["policy"])? != "pdf_docx_v1" {
        return Err(inconsistent());
    }
    Ok(StageSupportSnapshot {
        reference: DocumentVersionRef {
            id: DocumentId::from_uuid(uuid(&value["id"])?),
            version: DocumentVersion::new(counter(&value["version"])?)
                .map_err(|_| inconsistent())?,
        },
        digest: digest(&value["digest"])?,
        name: owned(&value["name"], 128)?,
        format: match string(&value["format"])? {
            "pdf" => StageDocumentFormat::Pdf,
            "docx" => StageDocumentFormat::Docx,
            _ => return Err(inconsistent()),
        },
        policy: StageFormatPolicy::PdfDocxV1,
    })
}
