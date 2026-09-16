use super::*;
use domain::{cases::CaseId, crypto::Sha256Digest};
impl From<ParticipantSnapshot> for ParticipantDetail {
    fn from(value: ParticipantSnapshot) -> Self {
        Self {
            revision: ParticipantRevisionSnapshot::Manual(Box::new(value)),
            bound_subject: None,
        }
    }
}
impl From<&ParticipantSnapshot> for ParticipantOverview {
    fn from(s: &ParticipantSnapshot) -> Self {
        Self {
            case_id: s.case_id,
            id: s.id,
            revision: s.revision,
            display_name: s.values.display_name().into(),
            procedural_role: s.values.procedural_role().into(),
            organization: s.values.organization().map(str::to_owned),
            directory_status: s.values.directory_status(),
            kind: None,
            subject: None,
        }
    }
}
impl From<ParticipantSnapshot> for ParticipantOverview {
    fn from(value: ParticipantSnapshot) -> Self {
        Self::from(&value)
    }
}
impl ParticipantDetail {
    pub fn case_id(&self) -> CaseId {
        match &self.revision {
            ParticipantRevisionSnapshot::Manual(v) => v.case_id,
            ParticipantRevisionSnapshot::Typed(v) => v.case_id,
        }
    }
    pub fn id(&self) -> ParticipantId {
        match &self.revision {
            ParticipantRevisionSnapshot::Manual(v) => v.id,
            ParticipantRevisionSnapshot::Typed(v) => v.id,
        }
    }
    pub fn revision_number(&self) -> ParticipantRevision {
        match &self.revision {
            ParticipantRevisionSnapshot::Manual(v) => v.revision,
            ParticipantRevisionSnapshot::Typed(v) => v.revision,
        }
    }
    pub fn values_digest(&self) -> Sha256Digest {
        match &self.revision {
            ParticipantRevisionSnapshot::Manual(v) => v.values_digest,
            ParticipantRevisionSnapshot::Typed(v) => v.values_digest,
        }
    }
    pub fn manual(&self) -> Option<&ParticipantSnapshot> {
        match &self.revision {
            ParticipantRevisionSnapshot::Manual(v) => Some(v),
            ParticipantRevisionSnapshot::Typed(_) => None,
        }
    }
    pub fn into_manual(self) -> Option<ParticipantSnapshot> {
        match self.revision {
            ParticipantRevisionSnapshot::Manual(v) => Some(*v),
            ParticipantRevisionSnapshot::Typed(_) => None,
        }
    }
}
