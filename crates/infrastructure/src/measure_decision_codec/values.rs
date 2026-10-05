use super::{primitives::*, temporal, Result};
use domain::{
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef},
    hearings::{HearingParticipantRef, HearingSupportRef},
    participants::{ParticipantId, ParticipantRevision},
    precautionary_measures::*,
    typed_participants::{CaseSubjectId, SubjectRevision, SubjectRevisionRef},
};
use serde_json::{json, Value};

pub fn decision_values(bytes: &[u8], projection: &Value) -> Result<MeasureDecisionValues> {
    frame(bytes, b"MDVAL1", 80, 16076)?;
    fields(
        projection,
        &[
            "authority",
            "declared_at",
            "justification",
            "support",
            "locator",
        ],
    )?;
    let support = &projection["support"];
    fields(support, &["document_id", "version", "digest"])?;
    let value = MeasureDecisionValues::new(MeasureDecisionValuesInput {
        authority: note(&projection["authority"])?,
        declared_at: temporal::decode(&projection["declared_at"])?,
        justification: note(&projection["justification"])?,
        support: HearingSupportRef::new(
            DocumentVersionRef {
                id: DocumentId::from_uuid(uuid(&support["document_id"])?),
                version: DocumentVersion::new(integer(&support["version"])?)
                    .map_err(|_| inconsistent())?,
            },
            digest(&support["digest"])?,
        ),
        locator: note(&projection["locator"])?,
    });
    if value.canonical_bytes() != bytes {
        return Err(inconsistent());
    }
    Ok(value)
}

pub fn measure_values(bytes: &[u8], projection: &Value) -> Result<MeasureValues> {
    frame(bytes, b"MEAS1", 91, 20113)?;
    let value = decode_measure(projection)?;
    if value.canonical_bytes() != bytes {
        return Err(inconsistent());
    }
    Ok(value)
}

pub(super) fn decode_measure(projection: &Value) -> Result<MeasureValues> {
    fields(
        projection,
        &["subject", "kind", "conditions", "validity", "supervision"],
    )?;
    let subject = &projection["subject"];
    fields(subject, &["id", "revision", "digest"])?;
    let validity = &projection["validity"];
    fields(validity, &["start", "statement", "end"])?;
    let supervision = &projection["supervision"];
    let supervision = match string(&supervision["kind"])? {
        "known" => {
            fields(supervision, &["kind", "participant", "statement"])?;
            let participant = &supervision["participant"];
            fields(participant, &["id", "revision"])?;
            MeasureSupervision::Known {
                participant: HearingParticipantRef::new(
                    ParticipantId::from_uuid(uuid(&participant["id"])?),
                    ParticipantRevision::new(integer(&participant["revision"])?)
                        .map_err(|_| inconsistent())?,
                ),
                statement: note(&supervision["statement"])?,
            }
        }
        "unknown" => {
            fields(supervision, &["kind", "reason"])?;
            MeasureSupervision::Unknown {
                reason: note(&supervision["reason"])?,
            }
        }
        _ => return Err(inconsistent()),
    };
    Ok(MeasureValues::new(MeasureValuesInput {
        subject: SubjectRevisionRef {
            id: CaseSubjectId::from_uuid(uuid(&subject["id"])?),
            revision: SubjectRevision::new(integer(&subject["revision"])?)
                .map_err(|_| inconsistent())?,
            values_digest: digest(&subject["digest"])?,
        },
        kind: string(&projection["kind"])?
            .parse()
            .map_err(|_| inconsistent())?,
        conditions: note(&projection["conditions"])?,
        validity: MeasureValidity::new(
            temporal::decode(&validity["start"])?,
            note(&validity["statement"])?,
            if validity["end"].is_null() {
                None
            } else {
                Some(temporal::decode(&validity["end"])?)
            },
        )
        .map_err(|_| inconsistent())?,
        supervision,
    }))
}

pub fn decision_view(value: &MeasureDecisionValues) -> Value {
    let support = value.support();
    json!({"authority":value.authority().as_str(),"declared_at":temporal::view(value.declared_at()),
        "justification":value.justification().as_str(),"support":{
            "document_id":support.reference().id.to_string(),"version":support.reference().version.get(),
            "digest":support.digest().to_hex()},"locator":value.locator().as_str()})
}

pub fn measure_view(value: &MeasureValues) -> Value {
    let subject = value.subject();
    let validity = value.validity();
    let supervision = match value.supervision() {
        MeasureSupervision::Known {
            participant,
            statement,
        } => json!({
            "kind":"known","participant":{"id":participant.id().to_string(),
            "revision":participant.revision().get()},"statement":statement.as_str()}),
        MeasureSupervision::Unknown { reason } => {
            json!({"kind":"unknown","reason":reason.as_str()})
        }
    };
    json!({"subject":{"id":subject.id.to_string(),"revision":subject.revision.get(),
        "digest":subject.values_digest.to_hex()},"kind":value.kind().as_str(),
        "conditions":value.conditions().as_str(),"validity":{
            "start":temporal::view(validity.start()),"statement":validity.statement().as_str(),
            "end":validity.end().map(temporal::view)},"supervision":supervision})
}
