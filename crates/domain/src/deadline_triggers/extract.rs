use super::{
    integrity, TriggerBlock as Block, TriggerExtraction, TriggerField as Field,
    TriggerIntegrityError as Error, TriggerMaterial as Material, TriggerOutcome as Outcome,
    TriggerRequirement as Requirement, TriggerSelection, TriggerSourceSnapshot,
};
use crate::{
    hearing_results::DeclaredHearingResultTime, judicial_calendars::CivilDate,
    procedural_facts::FactDeclaration, procedural_time::DeclaredProceduralTime,
};

/// Extract a required declared time while preserving source identity and evidence.
///
/// The caller verifies source hashes, receipts and authorization. This function
/// checks cross-material identity but does not establish legal applicability.
pub fn extract_trigger_time(
    requirement: Requirement,
    selection: &TriggerSelection,
    material: Option<Material<'_>>,
) -> Result<TriggerExtraction, Error> {
    let reference = match &selection.source {
        FactDeclaration::Unknown(_) => {
            if material.is_some() {
                return Err(Error::UnexpectedMaterial);
            }
            return Ok(TriggerExtraction {
                requirement,
                selection: selection.clone(),
                source: None,
                outcome: Outcome::Blocked(Block::UnknownSource),
            });
        }
        FactDeclaration::Known(reference) => *reference,
    };
    let material = material.ok_or(Error::MissingMaterial)?;
    let mut source = integrity::resolve(selection.case_id, reference, material)?;
    let outcome = if requirement.family() != reference.family() {
        Outcome::Blocked(Block::IncompatibleFamily {
            expected: requirement.family(),
            actual: reference.family(),
        })
    } else {
        extract(requirement, selection, material, &mut source)?
    };
    Ok(TriggerExtraction {
        requirement,
        selection: selection.clone(),
        source: Some(source),
        outcome,
    })
}

fn extract(
    requirement: Requirement,
    selection: &TriggerSelection,
    material: Material<'_>,
    source: &mut TriggerSourceSnapshot,
) -> Result<Outcome, Error> {
    match (requirement, &selection.qualification) {
        (Requirement::SourceField(_), Some(_)) => {
            Ok(Outcome::Blocked(Block::UnexpectedQualification))
        }
        (Requirement::SourceField(field), None) => field_time(field, material, source),
        (Requirement::Qualified { purpose, .. }, None) => {
            Ok(Outcome::Blocked(Block::MissingQualification { purpose }))
        }
        (Requirement::Qualified { purpose, .. }, Some(value)) => {
            if purpose != value.purpose {
                Ok(Outcome::Blocked(Block::QualificationMismatch {
                    expected: purpose,
                    actual: value.purpose,
                }))
            } else {
                Ok(Outcome::Extracted { at: value.at })
            }
        }
    }
}

fn field_time(
    field: Field,
    material: Material<'_>,
    source: &mut TriggerSourceSnapshot,
) -> Result<Outcome, Error> {
    let at = match (field, material) {
        (Field::ResolutionIssuedAt, Material::Resolution { values, .. }) => {
            Some(values.issued_at())
        }
        (Field::NotificationPracticedAt, Material::Notification { values, .. }) => {
            Some(values.practiced_at())
        }
        (Field::NotificationReceivedAt, Material::Notification { values, .. }) => {
            values.received_at()
        }
        (Field::NotificationStatedEffectAt, Material::Notification { values, .. }) => {
            source.stated_effect = values.stated_effect().cloned();
            values.stated_effect().map(|value| value.at)
        }
        (Field::HearingSessionEventTime, Material::HearingResult { values, .. }) => {
            Some(hearing_time(values.event_time())?)
        }
        _ => return Err(Error::SourceMismatch),
    };
    Ok(match at {
        Some(at) => Outcome::Extracted { at },
        None => Outcome::Blocked(Block::AbsentField(field)),
    })
}

fn hearing_time(value: DeclaredHearingResultTime) -> Result<DeclaredProceduralTime, Error> {
    let date = CivilDate::from_date(value.local_date()).map_err(|_| Error::InvalidTime)?;
    let offset = Some(value.offset());
    match value.instant_value() {
        Some(instant) => DeclaredProceduralTime::second(
            date,
            instant.hour(),
            instant.minute(),
            instant.second(),
            offset,
        ),
        None => DeclaredProceduralTime::date(date, offset),
    }
    .map_err(|_| Error::InvalidTime)
}
