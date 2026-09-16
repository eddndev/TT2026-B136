use super::{encoding::*, *};

impl ResolutionValues {
    /// PFRES1 binds declarations; resolved source snapshots and root identity require a receipt.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = b"PFRES1".to_vec();
        declaration(&mut bytes, self.class(), resolution_class);
        optional(&mut bytes, self.subtype(), |b, value| {
            text(b, value.as_str())
        });
        declaration(&mut bytes, self.issuer(), |b, value| {
            text(b, value.as_str())
        });
        declared_time(&mut bytes, self.issued_at());
        text(&mut bytes, self.summary().as_str());
        provenance(&mut bytes, self.provenance());
        bytes
    }
}

impl NotificationValues {
    /// PFNOT1 preserves every function and locator, including shared documentary supports.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = b"PFNOT1".to_vec();
        bytes.extend_from_slice(self.resolution().id.as_uuid().as_bytes());
        bytes.extend_from_slice(&self.resolution().revision.get().to_be_bytes());
        declaration(&mut bytes, self.character(), character);
        declaration(&mut bytes, self.medium(), medium);
        declaration(&mut bytes, self.context(), context);
        declaration(&mut bytes, self.outcome(), |b, value| {
            b.push(match value {
                NotificationOutcome::Practiced => 0,
                NotificationOutcome::Attempted => 1,
            })
        });
        optional(&mut bytes, self.subtype(), |b, value| {
            text(b, value.as_str())
        });
        declared_time(&mut bytes, self.practiced_at());
        optional(&mut bytes, self.received_at().as_ref(), |b, value| {
            declared_time(b, *value)
        });
        optional(&mut bytes, self.stated_effect(), |b, value| {
            declared_time(b, value.at);
            text(b, value.statement.as_str());
            text(b, value.locator.as_str());
        });
        declaration(&mut bytes, self.intended_recipient(), person);
        declaration(&mut bytes, self.actual_receiver(), person);
        representation(&mut bytes, self.representation());
        text(&mut bytes, self.summary().as_str());
        provenance(&mut bytes, self.provenance());
        bytes
    }
}

fn resolution_class(bytes: &mut Vec<u8>, value: &ResolutionClass) {
    match value {
        ResolutionClass::Order => bytes.push(0),
        ResolutionClass::Judgment => bytes.push(1),
        ResolutionClass::Other(label) => {
            bytes.push(2);
            text(bytes, label.as_str());
        }
    }
}
fn character(bytes: &mut Vec<u8>, value: &NotificationCharacter) {
    match value {
        NotificationCharacter::Personal => bytes.push(0),
        NotificationCharacter::Publication => bytes.push(1),
        NotificationCharacter::Other(label) => {
            bytes.push(2);
            text(bytes, label.as_str());
        }
    }
}
fn medium(bytes: &mut Vec<u8>, value: &NotificationMedium) {
    match value {
        NotificationMedium::InPerson => bytes.push(0),
        NotificationMedium::Electronic => bytes.push(1),
        NotificationMedium::Other(label) => {
            bytes.push(2);
            text(bytes, label.as_str());
        }
    }
}
fn context(bytes: &mut Vec<u8>, value: &NotificationContext) {
    match value {
        NotificationContext::InHearing => bytes.push(0),
        NotificationContext::OutsideHearing => bytes.push(1),
        NotificationContext::Other(label) => {
            bytes.push(2);
            text(bytes, label.as_str());
        }
    }
}
