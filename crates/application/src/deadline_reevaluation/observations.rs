use super::{validation, wire, wire::Reader, *};

type Result<T> = std::result::Result<T, TrackingCodecError>;

/// Commit exact observed references without replacing selected historical inputs.
pub fn encode_observations(value: &Observations) -> Result<Vec<u8>> {
    validate(value)?;
    let mut bytes = b"DLOB1".to_vec();
    wire::case_id(&mut bytes, value.case_id);
    bytes.push(value.entries.len() as u8);
    for entry in &value.entries {
        bytes.extend([entry.role as u8, entry.family as u8]);
        wire::uuid(&mut bytes, entry.id);
        bytes.extend_from_slice(&entry.revision.to_be_bytes());
        wire::optional(&mut bytes, entry.case_id, wire::case_id);
        wire::optional(&mut bytes, entry.hearing_id, wire::uuid);
        wire::optional(&mut bytes, entry.parent_resolution, |bytes, parent| {
            wire::uuid(bytes, parent.id);
            bytes.extend_from_slice(&parent.revision.to_be_bytes());
        });
        bytes.extend_from_slice(entry.submission_digest.as_bytes());
        bytes.extend_from_slice(entry.evidence_digest.as_bytes());
    }
    if bytes.len() > MAX_OBSERVATIONS_BYTES {
        return Err(TrackingCodecError::SizeLimit);
    }
    Ok(bytes)
}

/// Parse only DLOB1. Receipt and evidence digest verification belongs to the caller.
pub fn decode_observations(bytes: &[u8]) -> Result<Observations> {
    let mut reader = Reader::new(bytes, b"DLOB1", MAX_OBSERVATIONS_BYTES)?;
    let case_id = reader.case_id()?;
    let count = reader.byte()?;
    if !(1..=4).contains(&count) {
        return Err(TrackingCodecError::InvalidEncoding("observation count"));
    }
    let mut entries = Vec::with_capacity(usize::from(count));
    for _ in 0..count {
        let role = match reader.byte()? {
            0 => ObservationRole::Profile,
            1 => ObservationRole::Source,
            2 => ObservationRole::Calendar,
            3 => ObservationRole::NotificationParent,
            _ => return Err(TrackingCodecError::InvalidEncoding("observation role")),
        };
        entries.push(ObservationEntry {
            role,
            family: reader.family()?,
            id: reader.uuid()?,
            revision: reader.u32()?,
            case_id: reader.optional(Reader::case_id)?,
            hearing_id: reader.optional(Reader::uuid)?,
            parent_resolution: reader.optional(|reader| {
                Ok(ResolutionReference {
                    id: reader.uuid()?,
                    revision: reader.u32()?,
                })
            })?,
            submission_digest: reader.digest()?,
            evidence_digest: reader.digest()?,
        });
    }
    reader.finish()?;
    let value = Observations { case_id, entries };
    validate(&value)?;
    Ok(value)
}

fn validate(value: &Observations) -> Result<()> {
    if !(1..=4).contains(&value.entries.len())
        || value.entries[0].role != ObservationRole::Profile
        || value
            .entries
            .windows(2)
            .any(|pair| pair[0].role >= pair[1].role)
    {
        return Err(TrackingCodecError::InvalidShape(
            "observation roles and order",
        ));
    }
    for entry in &value.entries {
        if entry.revision == 0 {
            return Err(TrackingCodecError::InvalidShape("observation revision"));
        }
        validation::scope(value.case_id, entry.family, entry.case_id, entry.hearing_id)?;
        let role_family = match entry.role {
            ObservationRole::Profile => entry.family == DependencyFamily::Profile,
            ObservationRole::Calendar => entry.family == DependencyFamily::Calendar,
            ObservationRole::Source => matches!(
                entry.family,
                DependencyFamily::Resolution
                    | DependencyFamily::Notification
                    | DependencyFamily::HearingResult
            ),
            ObservationRole::NotificationParent => entry.family == DependencyFamily::Resolution,
        };
        if !role_family {
            return Err(TrackingCodecError::InvalidShape(
                "observation role and family",
            ));
        }
        match (entry.family, entry.parent_resolution) {
            (DependencyFamily::Notification, Some(parent)) if parent.revision > 0 => {}
            (DependencyFamily::Notification, _) | (_, Some(_)) => {
                return Err(TrackingCodecError::InvalidShape(
                    "notification parent reference",
                ));
            }
            _ => {}
        }
    }
    if let Some(parent) = value
        .entries
        .iter()
        .find(|entry| entry.role == ObservationRole::NotificationParent)
    {
        let source = value
            .entries
            .iter()
            .find(|entry| entry.role == ObservationRole::Source);
        let exact = source.and_then(|entry| entry.parent_resolution);
        if !exact.is_some_and(|exact| exact.id == parent.id && exact.revision <= parent.revision) {
            return Err(TrackingCodecError::InvalidShape("related parent head"));
        }
    }
    // Legacy captures may lack an observed parent head. Acceptance of new
    // qualifications must require it separately; the exact parent remains here.
    Ok(())
}
