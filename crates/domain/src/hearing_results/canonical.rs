use super::HearingResultValues;

impl HearingResultValues {
    /// HRES1 binds normalized declarations; immutable root sources belong to receipts.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = b"HRES1".to_vec();
        bytes.push(self.occurrence().tag());
        bytes.push(self.extent().tag());
        let event_time = self.event_time();
        if let Some(instant) = event_time.instant_value() {
            bytes.push(1);
            bytes.extend_from_slice(&instant.unix_timestamp().to_be_bytes());
        } else {
            bytes.push(0);
            bytes.extend_from_slice(&(event_time.local_date().year() as u16).to_be_bytes());
            bytes.push(event_time.local_date().month() as u8);
            bytes.push(event_time.local_date().day());
        }
        bytes.extend_from_slice(&event_time.offset().whole_seconds().to_be_bytes());
        text(&mut bytes, self.summary().as_str());
        bytes.push(self.attendees().len() as u8);
        for attendee in self.attendees() {
            bytes.extend_from_slice(attendee.participant_id().as_uuid().as_bytes());
            bytes.extend_from_slice(&attendee.revision().get().to_be_bytes());
            text(&mut bytes, attendee.capacity().as_str());
            optional_text(
                &mut bytes,
                attendee.observation().map(|value| value.as_str()),
            );
        }
        bytes.push(self.agreements().len() as u8);
        for agreement in self.agreements() {
            bytes.extend_from_slice(agreement.id().as_uuid().as_bytes());
            text(&mut bytes, agreement.text().as_str());
        }
        bytes.push(self.provenance().kind().tag());
        optional_text(
            &mut bytes,
            self.provenance().reference().map(|value| value.as_str()),
        );
        bytes.push(u8::from(self.provenance().support().is_some()));
        if let Some(support) = self.provenance().support() {
            bytes.extend_from_slice(support.reference().id.as_uuid().as_bytes());
            bytes.extend_from_slice(&support.reference().version.get().to_be_bytes());
            bytes.extend_from_slice(support.digest().as_bytes());
        }
        bytes
    }
}

fn text(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u32).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn optional_text(bytes: &mut Vec<u8>, value: Option<&str>) {
    bytes.push(u8::from(value.is_some()));
    if let Some(value) = value {
        text(bytes, value);
    }
}
