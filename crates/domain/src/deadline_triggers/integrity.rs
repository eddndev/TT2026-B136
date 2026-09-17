use super::{
    TriggerDigests, TriggerIntegrityError as Error, TriggerMaterial, TriggerProvenance,
    TriggerSourceRef, TriggerSourceSnapshot,
};
use crate::cases::CaseId;

/// Validate the supplied material before any semantic requirement can hide a mismatch.
pub(super) fn resolve(
    case_id: CaseId,
    reference: TriggerSourceRef,
    material: TriggerMaterial<'_>,
) -> Result<TriggerSourceSnapshot, Error> {
    if case_id != material.case_id() {
        return Err(Error::CaseMismatch);
    }
    let (digests, provenance, agreement) = match (reference, material) {
        (
            TriggerSourceRef::Resolution(selected),
            TriggerMaterial::Resolution {
                root,
                revision,
                values,
                digests,
            },
        ) => {
            if selected.id != root.id() || selected.revision != revision {
                return Err(Error::SourceMismatch);
            }
            (
                TriggerDigests::ProceduralFact(digests),
                TriggerProvenance::ProceduralFact(values.provenance().clone()),
                None,
            )
        }
        (
            TriggerSourceRef::Notification {
                id,
                revision: selected_revision,
                resolution,
            },
            TriggerMaterial::Notification {
                root,
                revision,
                values,
                digests,
            },
        ) => {
            if id != root.id() || selected_revision != revision {
                return Err(Error::SourceMismatch);
            }
            if resolution != values.resolution() || root.resolution_id() != resolution.id {
                return Err(Error::ParentMismatch);
            }
            (
                TriggerDigests::ProceduralFact(digests),
                TriggerProvenance::ProceduralFact(values.provenance().clone()),
                None,
            )
        }
        (
            TriggerSourceRef::HearingResult(selected),
            TriggerMaterial::HearingResult {
                hearing_id,
                result_id,
                revision,
                values,
                digests,
                ..
            },
        ) => {
            if selected.hearing_id != hearing_id
                || selected.result_id != result_id
                || selected.revision != revision
            {
                return Err(Error::SourceMismatch);
            }
            let agreement = selected
                .agreement_id
                .map(|id| {
                    values
                        .agreements()
                        .iter()
                        .find(|value| value.id() == id)
                        .cloned()
                        .ok_or(Error::MissingAgreement)
                })
                .transpose()?;
            (
                TriggerDigests::HearingResult(digests),
                TriggerProvenance::HearingResult(values.provenance().clone()),
                agreement,
            )
        }
        _ => return Err(Error::SourceMismatch),
    };
    Ok(TriggerSourceSnapshot {
        case_id,
        reference,
        digests,
        provenance,
        agreement,
        stated_effect: None,
    })
}
