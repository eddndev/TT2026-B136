use super::{encoding::blob, PrecautionaryContextMaterial};
use crate::{
    case_stages::{CaseStageEntry, StageSupportSnapshot},
    cases::{CaseActorSnapshot, CaseAdministrationSnapshot},
    documents::{StageDocumentFormat, StageFormatPolicy},
};
use domain::{case_stages::CaseStage, clock::OffsetDateTime};

pub(super) fn encode(material: &PrecautionaryContextMaterial) -> Vec<u8> {
    let mut bytes = b"PCTX1".to_vec();
    bytes.extend_from_slice(material.case_id.as_uuid().as_bytes());
    administration(&mut bytes, &material.administration);
    administration(&mut bytes, &material.stage_administration);
    let stage = &material.stage;
    bytes.push(match stage {
        CaseStageEntry::Initial(_) => 0,
        CaseStageEntry::Changed(_) => 1,
    });
    bytes.extend_from_slice(stage.case_id().as_uuid().as_bytes());
    bytes.extend_from_slice(&stage.stage_revision().get().to_be_bytes());
    bytes.push(stage_tag(stage.stage()));
    let (revision, digest) = match stage {
        CaseStageEntry::Initial(value) => {
            (value.administration_revision, value.administration_digest)
        }
        CaseStageEntry::Changed(value) => {
            (value.administration_revision, value.administration_digest)
        }
    };
    bytes.extend_from_slice(&revision.get().to_be_bytes());
    bytes.extend_from_slice(digest.as_bytes());
    timestamp(&mut bytes, stage.recorded_at());
    actor(&mut bytes, stage.recorded_by());
    if let CaseStageEntry::Changed(value) = stage {
        bytes.push(u8::from(value.from_stage.is_some()));
        if let Some(from) = value.from_stage {
            bytes.push(stage_tag(from));
        }
        blob(&mut bytes, &value.values.canonical_bytes());
        bytes.extend_from_slice(value.values_digest.as_bytes());
        bytes.extend_from_slice(&(value.supports.len() as u32).to_be_bytes());
        for support in &value.supports {
            document(&mut bytes, support);
        }
    }
    bytes
}

fn administration(bytes: &mut Vec<u8>, value: &CaseAdministrationSnapshot) {
    bytes.extend_from_slice(value.case_id.as_uuid().as_bytes());
    bytes.extend_from_slice(&value.revision.get().to_be_bytes());
    blob(bytes, &value.values.canonical_bytes());
    bytes.extend_from_slice(value.values_digest.as_bytes());
    timestamp(bytes, value.changed_at);
    actor(bytes, &value.changed_by);
}

pub(crate) fn timestamp(bytes: &mut Vec<u8>, at: OffsetDateTime) {
    bytes.extend_from_slice(&at.unix_timestamp().to_be_bytes());
    bytes.extend_from_slice(&at.nanosecond().to_be_bytes());
    bytes.extend_from_slice(&at.offset().whole_seconds().to_be_bytes());
}

fn actor(bytes: &mut Vec<u8>, actor: &CaseActorSnapshot) {
    bytes.extend_from_slice(actor.id.as_uuid().as_bytes());
    blob(bytes, actor.email.as_bytes());
}

fn stage_tag(value: CaseStage) -> u8 {
    match value {
        CaseStage::Investigation => 0,
        CaseStage::Intermediate => 1,
        CaseStage::Trial => 2,
    }
}

pub(crate) fn document(bytes: &mut Vec<u8>, support: &StageSupportSnapshot) {
    bytes.extend_from_slice(support.reference.id.as_uuid().as_bytes());
    bytes.extend_from_slice(&support.reference.version.get().to_be_bytes());
    bytes.extend_from_slice(support.digest.as_bytes());
    blob(bytes, support.name.as_bytes());
    bytes.push(match support.format {
        StageDocumentFormat::Pdf => 0,
        StageDocumentFormat::Docx => 1,
    });
    bytes.push(match support.policy {
        StageFormatPolicy::PdfDocxV1 => 0,
    });
}
