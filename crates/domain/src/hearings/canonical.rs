use super::HearingValues;

impl HearingValues {
    /// HEAR1 binds normalized appointment values, excluding organizational metadata.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = b"HEAR1".to_vec();
        bytes.push(self.kind().tag());
        bytes.extend_from_slice(&self.scheduled_at().value().unix_timestamp().to_be_bytes());
        bytes.extend_from_slice(
            &self
                .scheduled_at()
                .value()
                .offset()
                .whole_seconds()
                .to_be_bytes(),
        );
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
        bytes.push(u8::from(self.conviction_basis().is_some()));
        if let Some(basis) = self.conviction_basis() {
            text(&mut bytes, basis.statement().as_str());
            bytes.extend_from_slice(basis.support().reference().id.as_uuid().as_bytes());
            bytes.extend_from_slice(&basis.support().reference().version.get().to_be_bytes());
            bytes.extend_from_slice(basis.support().digest().as_bytes());
        }
        bytes
    }
}

fn text(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u32).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}
