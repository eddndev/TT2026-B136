use super::{
    declarations::{class, source_declaration},
    helpers::*,
    inconsistent, source_entries, temporal, Result,
};
use application::procedural_facts::*;
use domain::cases::CaseId;
use serde_json::Value;

fn resolution(value: &Value) -> Result<(FactResolutionSourceSnapshot, FactResolutionView)> {
    fields(
        value,
        &[
            "case",
            "id",
            "revision",
            "values_digest",
            "submission_digest",
            "status",
            "class",
            "issuer",
            "issued_at",
            "summary",
        ],
    )?;
    let reference = FactResolutionRef {
        id: ResolutionId::from_uuid(uuid(&value["id"])?),
        revision: FactRevision::new(counter(&value["revision"])?).map_err(|_| inconsistent())?,
    };
    Ok((
        FactResolutionSourceSnapshot {
            case_id: CaseId::from_uuid(uuid(&value["case"])?),
            reference,
            values_digest: digest(&value["values_digest"])?,
            submission_digest: digest(&value["submission_digest"])?,
            status: match string(&value["status"])? {
                "recorded" => FactStatus::Recorded,
                "withdrawn" => FactStatus::Withdrawn,
                _ => return Err(inconsistent()),
            },
        },
        FactResolutionView {
            reference,
            class: source_declaration(&value["class"], class)?,
            issuer: source_declaration(&value["issuer"], label)?,
            issued_at: temporal::source_time(&value["issued_at"])?,
            summary: text(&value["summary"])?,
        },
    ))
}
pub(super) fn decode(value: &Value) -> Result<FactSources> {
    fields(
        value,
        &[
            "resolution",
            "participants",
            "hearing_results",
            "direct_supports",
        ],
    )?;
    let parent = optional(&value["resolution"], resolution)?;
    let participants = array(&value["participants"], 4)?
        .iter()
        .map(source_entries::participant)
        .collect::<Result<Vec<_>>>()?;
    let hearings = array(&value["hearing_results"], 2)?
        .iter()
        .map(source_entries::hearing)
        .collect::<Result<Vec<_>>>()?;
    let documents = array(&value["direct_supports"], 2)?
        .iter()
        .map(source_entries::support)
        .collect::<Result<Vec<_>>>()?;
    let (participants, participant_views) = participants.into_iter().unzip();
    let (hearing_results, hearing_views) = hearings.into_iter().unzip();
    Ok(FactSources {
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
        direct_supports: documents,
    })
}
