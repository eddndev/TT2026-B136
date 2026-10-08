use super::{
    canonical::{measure_time, text},
    MeasureDecisionValues, MeasureSupervision, MeasureValues,
};

impl MeasureValues {
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = b"MEAS1".to_vec();
        let subject = self.subject();
        bytes.extend_from_slice(subject.id.as_uuid().as_bytes());
        bytes.extend_from_slice(&subject.revision.get().to_be_bytes());
        bytes.extend_from_slice(subject.values_digest.as_bytes());
        bytes.push(self.kind().tag());
        text(&mut bytes, self.conditions().as_str());
        let validity = self.validity().canonical_bytes();
        bytes.extend_from_slice(&(validity.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&validity);
        match self.supervision() {
            MeasureSupervision::Known {
                participant,
                statement,
            } => {
                bytes.push(0);
                bytes.extend_from_slice(participant.id().as_uuid().as_bytes());
                bytes.extend_from_slice(&participant.revision().get().to_be_bytes());
                text(&mut bytes, statement.as_str());
            }
            MeasureSupervision::Unknown { reason } => {
                bytes.push(1);
                text(&mut bytes, reason.as_str());
            }
        }
        bytes
    }
}

impl MeasureDecisionValues {
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = b"MDVAL1".to_vec();
        text(&mut bytes, self.authority().as_str());
        measure_time(&mut bytes, self.declared_at());
        text(&mut bytes, self.justification().as_str());
        let support = self.support();
        bytes.extend_from_slice(support.reference().id.as_uuid().as_bytes());
        bytes.extend_from_slice(&support.reference().version.get().to_be_bytes());
        bytes.extend_from_slice(support.digest().as_bytes());
        text(&mut bytes, self.locator().as_str());
        bytes
    }
}
