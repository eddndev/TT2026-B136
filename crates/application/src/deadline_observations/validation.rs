use super::inconsistent;
use crate::{
    cases::case_administration_digest,
    deadline_inputs::{DeadlineInputMaterial, DeadlineSourceDetail},
    deadlines::evidence,
    hearing_results::hearing_result_receipt_matches,
    judicial_calendars::judicial_calendar_receipt_matches,
    procedural_facts::{
        fact_receipt_matches, FactDetail, FactResolutionSourceSnapshot, FactResolutionView,
        ProceduralFactSnapshot,
    },
    ApplicationError,
};
use domain::{cases::CaseId, crypto::DocumentHasher};

type Result<T> = std::result::Result<T, ApplicationError>;

pub(super) fn material(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    value: &DeadlineInputMaterial,
) -> Result<()> {
    if value.case_id != case_id {
        return Err(inconsistent("observed material belongs to another case"));
    }
    if let Some(snapshot) = value.administration.snapshot() {
        if snapshot.case_id != case_id
            || case_administration_digest(hasher, &snapshot.values) != snapshot.values_digest
        {
            return Err(inconsistent(
                "observed administration scope or digest differs",
            ));
        }
    }
    sources(
        hasher,
        case_id,
        value.source.as_ref(),
        value.source_head.as_ref(),
    )?;
    match (&value.calendar, &value.calendar_head) {
        (None, None) => {}
        (Some(exact), Some(head)) => {
            judicial_calendar_receipt_matches(hasher, exact)?;
            judicial_calendar_receipt_matches(hasher, head)?;
            if exact.id != head.id
                || exact.revision > head.revision
                || exact.values.scope() != head.values.scope()
                || (exact.revision == head.revision
                    && (exact != head || calendar_bytes(exact) != calendar_bytes(head)))
            {
                return Err(inconsistent(
                    "calendar head identity, scope, revision or metadata differs",
                ));
            }
        }
        _ => return Err(inconsistent("calendar and head presence differ")),
    }
    Ok(())
}

fn sources(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    exact: Option<&DeadlineSourceDetail>,
    head: Option<&DeadlineSourceDetail>,
) -> Result<()> {
    let (exact, head) = match (exact, head) {
        (None, None) => return Ok(()),
        (Some(exact), Some(head)) => (exact, head),
        _ => return Err(inconsistent("source and head presence differ")),
    };
    let order = match (exact, head) {
        (DeadlineSourceDetail::Fact(exact), DeadlineSourceDetail::Fact(head)) => {
            fact_receipt_matches(hasher, exact)?;
            fact_receipt_matches(hasher, head)?;
            if exact.snapshot.case_id() != case_id
                || head.snapshot.case_id() != case_id
                || exact.snapshot.target() != head.snapshot.target()
            {
                return Err(inconsistent("fact head identity or case differs"));
            }
            head.snapshot
                .metadata()
                .revision
                .cmp(&exact.snapshot.metadata().revision)
        }
        (DeadlineSourceDetail::HearingResult(exact), DeadlineSourceDetail::HearingResult(head)) => {
            hearing_result_receipt_matches(hasher, exact)?;
            hearing_result_receipt_matches(hasher, head)?;
            if exact.snapshot.case_id != case_id
                || head.snapshot.case_id != case_id
                || exact.snapshot.hearing_id != head.snapshot.hearing_id
                || exact.snapshot.id != head.snapshot.id
            {
                return Err(inconsistent(
                    "result head identity, hearing or case differs",
                ));
            }
            head.snapshot.revision.cmp(&exact.snapshot.revision)
        }
        _ => return Err(inconsistent("source and head families differ")),
    };
    if order.is_lt()
        || (order.is_eq()
            && (exact != head || source_bytes(hasher, exact) != source_bytes(hasher, head)))
    {
        return Err(inconsistent(
            "source head revision or exact metadata differs",
        ));
    }
    Ok(())
}

pub(super) fn parent(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    material: &DeadlineInputMaterial,
    parent: Option<&FactDetail>,
) -> Result<()> {
    let notification = matches!(&material.source_head,
        Some(DeadlineSourceDetail::Fact(detail)) if matches!(detail.snapshot, ProceduralFactSnapshot::Notification(_)));
    let parent = match (notification, parent) {
        (false, None) => return Ok(()),
        (true, Some(parent)) => parent,
        _ => return Err(inconsistent("notification parent head presence differs")),
    };
    fact_receipt_matches(hasher, parent)?;
    let ProceduralFactSnapshot::Resolution(parent) = &parent.snapshot else {
        return Err(inconsistent(
            "notification parent head must be a resolution",
        ));
    };
    if parent.root.case_id() != case_id {
        return Err(inconsistent(
            "notification parent head belongs to another case",
        ));
    }
    for source in [&material.source, &material.source_head] {
        let Some(DeadlineSourceDetail::Fact(detail)) = source else {
            return Err(inconsistent("notification source is absent"));
        };
        let ProceduralFactSnapshot::Notification(notification) = &detail.snapshot else {
            return Err(inconsistent("notification source has another family"));
        };
        let exact = notification.values.resolution();
        if parent.root.id() != exact.id || parent.metadata.revision < exact.revision {
            return Err(inconsistent(
                "notification parent head identity or revision differs",
            ));
        }
        if parent.metadata.revision == exact.revision {
            let expected = FactResolutionSourceSnapshot {
                case_id,
                reference: exact,
                values_digest: parent.metadata.values_digest,
                submission_digest: parent.metadata.receipt.submission_digest,
                status: parent.metadata.status,
            };
            let view = FactResolutionView {
                reference: exact,
                class: parent.values.class().clone(),
                issuer: parent.values.issuer().clone(),
                issued_at: parent.values.issued_at(),
                summary: parent.values.summary().clone(),
            };
            if detail.sources.resolved.resolution != Some(expected)
                || detail.sources.views.resolution.as_ref() != Some(&view)
            {
                return Err(inconsistent("exact notification parent projection differs"));
            }
        }
    }
    Ok(())
}

fn calendar_bytes(value: &crate::judicial_calendars::JudicialCalendarDetail) -> Vec<u8> {
    let mut bytes = Vec::new();
    evidence::calendar(&mut bytes, value);
    bytes
}

fn source_bytes(hasher: &dyn DocumentHasher, value: &DeadlineSourceDetail) -> Vec<u8> {
    let mut bytes = Vec::new();
    evidence::source(&mut bytes, hasher, Some(value));
    bytes
}
