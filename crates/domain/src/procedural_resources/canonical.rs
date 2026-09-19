use super::{encoding::*, ResourceActValues, ResourceValues};

impl ResourceValues {
    /// PRSC1 binds declarations and selections; root, author and verified sources require a receipt.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = b"PRSC1".to_vec();
        bytes.push(self.kind().tag());
        declaration(&mut bytes, self.mode(), |bytes, mode| {
            bytes.push(mode.tag())
        });
        text(&mut bytes, self.title().as_str());
        resolution(&mut bytes, self.resolution());
        evidence(&mut bytes, self.resolution_evidence());
        declaration(&mut bytes, self.resolution_reference(), |bytes, value| {
            text(bytes, value.as_str())
        });
        declaration(&mut bytes, self.issuing_authority(), |bytes, value| {
            text(bytes, value.as_str())
        });
        optional(&mut bytes, self.receiving_authority(), |bytes, value| {
            declaration(bytes, value, |bytes, label| text(bytes, label.as_str()));
        });
        declared_time(&mut bytes, self.resolution_at());
        optional(
            &mut bytes,
            self.notification_at().as_ref(),
            |bytes, value| declared_time(bytes, *value),
        );
        text(&mut bytes, self.challenged_part().as_str());
        text(&mut bytes, self.grounds().as_str());
        bytes.push(self.appellants().len() as u8);
        for appellant in self.appellants() {
            text(&mut bytes, appellant.name().as_str());
            declaration(&mut bytes, appellant.role(), |bytes, role| {
                text(bytes, role.as_str())
            });
            optional(
                &mut bytes,
                appellant.participant().as_ref(),
                |bytes, reference| {
                    bytes.extend_from_slice(reference.id.as_uuid().as_bytes());
                    bytes.extend_from_slice(&reference.revision.get().to_be_bytes());
                },
            );
        }
        bytes
    }
}

impl ResourceActValues {
    /// PRAC1 binds a declared act; recording it does not change organizational status.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = b"PRAC1".to_vec();
        bytes.push(self.kind().tag());
        declaration(&mut bytes, self.mode(), |bytes, mode| {
            bytes.push(mode.tag())
        });
        declared_time(&mut bytes, self.occurred_at());
        declaration(&mut bytes, self.authority(), |bytes, value| {
            text(bytes, value.as_str())
        });
        text(&mut bytes, self.statement().as_str());
        bytes.push(self.evidence().len() as u8);
        for value in self.evidence() {
            evidence(&mut bytes, value);
        }
        bytes
    }
}
