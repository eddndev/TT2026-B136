use super::ResourceHearingValues;

impl ResourceHearingValues {
    /// RHEAR1 commits normalized values and exact declared support, without receipts.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = b"RHEAR1".to_vec();
        bytes.push(self.kind().tag());
        let at = self.scheduled_at().value();
        bytes.extend_from_slice(&at.unix_timestamp().to_be_bytes());
        bytes.extend_from_slice(&at.offset().whole_seconds().to_be_bytes());
        bytes.push(self.modality().tag());
        text(&mut bytes, self.venue().as_str());
        bytes.push(u8::from(self.note().is_some()));
        if let Some(note) = self.note() {
            text(&mut bytes, note.as_str());
        }
        bytes.push(self.participants().len() as u8);
        for participant in self.participants() {
            bytes.extend_from_slice(participant.id().as_uuid().as_bytes());
            bytes.extend_from_slice(&participant.revision().get().to_be_bytes());
        }
        let basis = self.scheduling_basis();
        text(&mut bytes, basis.statement().as_str());
        let support = basis.support();
        bytes.extend_from_slice(support.reference().id.as_uuid().as_bytes());
        bytes.extend_from_slice(&support.reference().version.get().to_be_bytes());
        bytes.extend_from_slice(support.digest().as_bytes());
        bytes
    }
}

fn text(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u32).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}
