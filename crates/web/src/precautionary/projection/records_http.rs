use super::{administrative_http, common, history, measure};
use crate::error::ApiError;
use application::{measure_corrections::*, precautionary_measures::*};
use domain::{
    cases::CaseId,
    crypto::DocumentHasher,
    precautionary_hearings::{MeasureId, PrecautionaryMeasureRef},
};
use serde_json::{json, Value};

pub(crate) fn record_detail(
    value: &MeasureRecordDetail,
    hasher: &dyn DocumentHasher,
) -> Result<Value, ApiError> {
    bound_record(value, value.case_id, None, None)?;
    let (family, validity, action, root, origin, last) = match &value.record {
        OwnedMeasureRecord::Judicial(judicial) => {
            let family = match judicial {
                OwnedJudicialMeasure::V1(_) => "m1",
                OwnedJudicialMeasure::V2(_) => "m2",
            };
            (
                family,
                "valid",
                judicial.last_action(),
                judicial.record_root(),
                judicial.judicial_origin(),
                json!({"owner":common::group_ref(judicial.owner()),
                    "reference":common::reference(judicial.reference())}),
            )
        }
        OwnedMeasureRecord::Administrative { capture, .. } => {
            let result = &capture.result;
            let validity = match result.validity {
                MeasureCaptureValidity::Valid => "valid",
                MeasureCaptureValidity::EnteredInError => "entered_in_error",
            };
            (
                "c1",
                validity,
                result.last_action,
                result.record_root.clone(),
                result.judicial_origin,
                json!({"owner":common::group_ref(&result.last_judicial.owner),
                    "reference":common::reference(result.last_judicial.reference)}),
            )
        }
    };
    Ok(
        json!({"case_id":value.case_id.to_string(),"reference":common::reference(value.reference),
        "family":family,"validity":validity,"last_action":common::action(action),
        "record_root":measure::root(&root),"judicial_origin":common::origin_ids(origin),
        "last_judicial":last,"record":measure::owned_record(&value.record,hasher)?,
        "record_history":history::project(&value.record_history,hasher)?}),
    )
}

pub(crate) fn bound_record(
    value: &MeasureRecordDetail,
    case: CaseId,
    id: Option<MeasureId>,
    reference: Option<PrecautionaryMeasureRef>,
) -> Result<(), ApiError> {
    history::check(&value.record_history)?;
    if value.case_id != case
        || id.is_some_and(|id| id != value.reference.id())
        || reference.is_some_and(|reference| reference != value.reference)
    {
        return Err(ApiError::internal());
    }
    match &value.record {
        OwnedMeasureRecord::Judicial(judicial) => {
            if judicial.case_id() != case || judicial.reference() != value.reference {
                return Err(ApiError::internal());
            }
            match judicial {
                OwnedJudicialMeasure::V1(record) => bound_v1(value, record),
                OwnedJudicialMeasure::V2(record) => bound_v2(value, record),
            }
        }
        OwnedMeasureRecord::Administrative { owner, capture } => {
            if capture.case_id != case
                || administrative_http::reference(capture) != value.reference
                || capture.operation_id != owner.operation_id
            {
                return Err(ApiError::internal());
            }
            let mut matches = value
                .record_history
                .records
                .administrative
                .iter()
                .filter(|entry| entry.origin.operation_id == owner.operation_id);
            let entry = matches.next().ok_or_else(ApiError::internal)?;
            if matches.next().is_some() || entry.capture.capture_digest != owner.capture_digest {
                return Err(ApiError::internal());
            }
            administrative_http::bound_capture(&entry.capture, &entry.origin, case)?;
            let mut members = entry.capture.records.iter().filter(|row| {
                row.result.id == value.reference.id()
                    && row.result.revision == value.reference.revision()
            });
            if members.next() != Some(capture.as_ref()) || members.next().is_some() {
                return Err(ApiError::internal());
            }
            Ok(())
        }
    }
}

fn bound_v1(value: &MeasureRecordDetail, record: &OwnedMeasureMaterial) -> Result<(), ApiError> {
    let mut matches = value
        .record_history
        .records
        .judicial
        .groups
        .iter()
        .filter(|entry| entry.origin.operation_id == record.owner.operation_id);
    let entry = matches.next().ok_or_else(ApiError::internal)?;
    let group = &entry.capture;
    let review = &group.review;
    let origin = MeasureGroupOrigin {
        case_id: review.case_id,
        operation_id: review.command.operation_id,
        decision_id: review.command.decision_id,
        submission_digest: review.submission_digest,
        review_digest: review.review_digest,
        decision_digest: group.decision.capture_digest,
        group_digest: group.capture_digest,
    };
    if matches.next().is_some()
        || entry.origin != origin
        || origin.case_id != value.case_id
        || origin.operation_id != record.owner.operation_id
        || origin.decision_id != record.owner.decision_id
        || origin.group_digest != record.owner.group_digest
        || record.capture.operation_id != origin.operation_id
        || record.capture.decision_id != origin.decision_id
        || record.capture.decision_digest != origin.decision_digest
        || group.decision.case_id != value.case_id
        || group.decision.operation_id != origin.operation_id
        || group.decision.decision_id != origin.decision_id
    {
        return Err(ApiError::internal());
    }
    let mut members = group.measures.iter().filter(|row| {
        row.result.id == value.reference.id() && row.result.revision == value.reference.revision()
    });
    if members.next() != Some(&record.capture) || members.next().is_some() {
        return Err(ApiError::internal());
    }
    Ok(())
}

fn bound_v2(value: &MeasureRecordDetail, record: &OwnedMeasureMaterialV2) -> Result<(), ApiError> {
    let mut matches = value
        .record_history
        .decisions
        .iter()
        .filter(|entry| entry.origin.operation_id == record.owner.operation_id);
    let entry = matches.next().ok_or_else(ApiError::internal)?;
    let group = &entry.capture;
    let review = &group.review;
    let origin = MeasureGroupOrigin {
        case_id: review.case_id,
        operation_id: review.command.operation_id,
        decision_id: review.command.decision_id,
        submission_digest: review.submission_digest,
        review_digest: review.review_digest,
        decision_digest: group.decision.capture_digest,
        group_digest: group.capture_digest,
    };
    if matches.next().is_some()
        || entry.origin != origin
        || origin.case_id != value.case_id
        || origin.operation_id != record.owner.operation_id
        || origin.decision_id != record.owner.decision_id
        || origin.group_digest != record.owner.group_digest
        || record.capture.operation_id != origin.operation_id
        || record.capture.decision_id != origin.decision_id
        || record.capture.decision_digest != origin.decision_digest
        || group.decision.case_id != value.case_id
        || group.decision.operation_id != origin.operation_id
        || group.decision.decision_id != origin.decision_id
    {
        return Err(ApiError::internal());
    }
    let mut members = group.measures.iter().filter(|row| {
        row.result.id == value.reference.id() && row.result.revision == value.reference.revision()
    });
    if members.next() != Some(&record.capture) || members.next().is_some() {
        return Err(ApiError::internal());
    }
    Ok(())
}
