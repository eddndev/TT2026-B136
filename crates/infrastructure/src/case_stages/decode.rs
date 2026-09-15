use super::inconsistent;
use application::case_stages::*;
use application::cases::{CaseActorSnapshot, CaseInitialStageRegistration};
use application::ApplicationError;
use domain::case_administration::{CaseRevision, InitialCaseStage};
use domain::cases::CaseId;
use domain::crypto::{
    ArchiveEntry, DocumentHasher, DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest,
};
use domain::identity::UserId;
use postgres::Row;
use time::{
    format_description::well_known::Rfc3339, macros::format_description, Date, OffsetDateTime,
    UtcOffset,
};

pub(super) fn digest(bytes: Vec<u8>) -> Result<Sha256Digest, ApplicationError> {
    Ok(Sha256Digest::from_array(bytes.try_into().map_err(
        |_| inconsistent("invalid stage digest length"),
    )?))
}
fn counter(value: i64) -> Result<u32, ApplicationError> {
    u32::try_from(value).map_err(inconsistent)
}
fn actor(row: &Row) -> Result<CaseActorSnapshot, ApplicationError> {
    let email: String = row.get("recorded_by_email");
    if email.is_empty() || email.trim() != email || email.chars().any(char::is_control) {
        return Err(inconsistent("noncanonical captured stage actor"));
    }
    Ok(CaseActorSnapshot {
        id: UserId::from_uuid(row.get("recorded_by")),
        email,
    })
}
pub(super) fn initial(row: &Row) -> Result<CaseInitialStageRegistration, ApplicationError> {
    if !row.get::<_, bool>("canonical")
        || row.get::<_, i64>("stage_revision") != 1
        || row.get::<_, i64>("administration_revision") != 1
        || row.get::<_, &str>("stage") != "investigation"
    {
        return Err(inconsistent("invalid initial stage registration"));
    }
    let text: &str = row.get("recorded_at_text");
    let at = OffsetDateTime::parse(text, &Rfc3339).map_err(inconsistent)?;
    if at
        .to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .map_err(inconsistent)?
        != text
    {
        return Err(inconsistent("noncanonical initial stage time"));
    }
    Ok(CaseInitialStageRegistration {
        case_id: CaseId::from_uuid(row.get("case_id")),
        stage_revision: CaseStageRevision::FIRST,
        administration_revision: CaseRevision::FIRST,
        stage: InitialCaseStage::Investigation,
        administration_digest: digest(row.get("administration_digest"))?,
        recorded_at: at,
        recorded_by: actor(row)?,
    })
}

pub(crate) fn changed(
    row: &Row,
    hasher: &dyn DocumentHasher,
) -> Result<CaseStageSnapshot, ApplicationError> {
    if !row.get::<_, bool>("canonical")
        || !row.get::<_, bool>("recording_valid")
        || !row.get::<_, bool>("administration_valid")
    {
        return Err(inconsistent(
            "stored stage values or context are not canonical",
        ));
    }
    let primary = support(row, "support")?;
    let primary_ref = StageSupportRef::new(primary.reference, primary.digest);
    let time = declared(row, "act")?;
    let stage: CaseStage = row.get::<_, &str>("stage").parse().map_err(inconsistent)?;
    let note = note(row.get("note"))?;
    let mut supports = vec![primary];
    let values = match row.get::<_, &str>("change_kind") {
        "adoption" => CaseStageChange::Adopt(StageAdoption::new(
            stage,
            time,
            note_required(row.get("reason"))?,
            primary_ref,
        )),
        "to_intermediate" => {
            CaseStageChange::Transition(StageTransition::to_intermediate(time, primary_ref, note))
        }
        "to_trial" => {
            let court_text: &str = row.get("receiving_court");
            let court = StageCourt::new(court_text).map_err(inconsistent)?;
            if court.as_str() != court_text {
                return Err(inconsistent("noncanonical stage court"));
            }
            let raw: Option<&str> = row.get("receipt_reference");
            let reference = StageReceiptReference::optional(raw).map_err(inconsistent)?;
            if reference.as_ref().map(StageReceiptReference::as_str) != raw {
                return Err(inconsistent("noncanonical receipt reference"));
            }
            let receipt = if row.get::<_, Option<uuid::Uuid>>("receipt_id").is_some() {
                let snapshot = support(row, "receipt")?;
                let reference = StageSupportRef::new(snapshot.reference, snapshot.digest);
                if snapshot.reference != primary_ref.reference() {
                    supports.push(snapshot);
                } else if snapshot != supports[0] {
                    return Err(inconsistent(
                        "duplicate support roles disagree on captured format",
                    ));
                }
                Some(reference)
            } else {
                None
            };
            CaseStageChange::Transition(
                StageTransition::to_trial(
                    time,
                    primary_ref,
                    declared(row, "received")?,
                    court,
                    reference,
                    receipt,
                    note,
                )
                .map_err(inconsistent)?,
            )
        }
        _ => return Err(inconsistent("invalid stage change kind")),
    };
    let expected_digest = digest(row.get("values_digest"))?;
    if values.stage() != stage || case_stage_digest(hasher, &values) != expected_digest {
        return Err(inconsistent("stage canonical digest mismatch"));
    }
    let at = instant(
        row.get("recorded_at_seconds"),
        row.get("recorded_at_nanoseconds"),
    )?;
    values.validate_recording_at(at).map_err(inconsistent)?;
    Ok(CaseStageSnapshot {
        case_id: CaseId::from_uuid(row.get("case_id")),
        stage_revision: CaseStageRevision::new(counter(row.get("revision"))?)
            .map_err(inconsistent)?,
        from_stage: row
            .get::<_, Option<&str>>("from_stage")
            .map(str::parse)
            .transpose()
            .map_err(inconsistent)?,
        values,
        values_digest: expected_digest,
        administration_revision: CaseRevision::new(counter(row.get("administration_revision"))?)
            .map_err(inconsistent)?,
        administration_digest: digest(row.get("administration_digest"))?,
        supports,
        recorded_at: at,
        recorded_by: actor(row)?,
    })
}

fn note(raw: Option<&str>) -> Result<Option<StageNote>, ApplicationError> {
    let note = StageNote::optional(raw).map_err(inconsistent)?;
    if note.as_ref().map(StageNote::as_str) != raw {
        return Err(inconsistent("noncanonical stage note"));
    }
    Ok(note)
}
fn note_required(raw: Option<&str>) -> Result<StageNote, ApplicationError> {
    note(raw)?.ok_or_else(|| inconsistent("required stage reason is absent"))
}
fn support(row: &Row, prefix: &str) -> Result<StageSupportSnapshot, ApplicationError> {
    let name: String = row.get(format!("{prefix}_name").as_str());
    ArchiveEntry::new(name.clone(), Vec::new()).map_err(inconsistent)?;
    let format = match row.get::<_, &str>(format!("{prefix}_format").as_str()) {
        "pdf" => StageDocumentFormat::Pdf,
        "docx" => StageDocumentFormat::Docx,
        _ => return Err(inconsistent("unknown admitted support format")),
    };
    if row.get::<_, &str>(format!("{prefix}_policy").as_str()) != "pdf_docx_v1" {
        return Err(inconsistent("unknown support validation policy"));
    }
    Ok(StageSupportSnapshot {
        reference: DocumentVersionRef {
            id: DocumentId::from_uuid(row.get(format!("{prefix}_id").as_str())),
            version: DocumentVersion::new(counter(row.get(format!("{prefix}_version").as_str()))?)
                .map_err(inconsistent)?,
        },
        digest: digest(row.get(format!("{prefix}_digest").as_str()))?,
        name,
        format,
        policy: StageFormatPolicy::PdfDocxV1,
    })
}
fn declared(row: &Row, prefix: &str) -> Result<DeclaredStageTime, ApplicationError> {
    let offset =
        UtcOffset::from_whole_seconds(row.get(format!("{prefix}_offset_seconds").as_str()))
            .map_err(inconsistent)?;
    match row.get::<_, &str>(format!("{prefix}_precision").as_str()) {
        "date" => {
            let date = Date::parse(
                row.get(format!("{prefix}_date_text").as_str()),
                format_description!("[year]-[month]-[day]"),
            )
            .map_err(inconsistent)?;
            DeclaredStageTime::date(date, offset).map_err(inconsistent)
        }
        "instant" => {
            let utc = instant(
                row.get(format!("{prefix}_seconds").as_str()),
                row.get(format!("{prefix}_nanoseconds").as_str()),
            )?;
            DeclaredStageTime::instant(
                utc.checked_to_offset(offset)
                    .ok_or_else(|| inconsistent("local stage time is outside supported years"))?,
            )
            .map_err(inconsistent)
        }
        _ => Err(inconsistent("unknown declared stage precision")),
    }
}
fn instant(seconds: i64, nanos: i32) -> Result<OffsetDateTime, ApplicationError> {
    if !(0..=999_999_999).contains(&nanos) {
        return Err(inconsistent("invalid stage time nanoseconds"));
    }
    let time = OffsetDateTime::from_unix_timestamp_nanos(
        i128::from(seconds) * 1_000_000_000 + i128::from(nanos),
    )
    .map_err(inconsistent)?;
    if !(1..=9999).contains(&time.year()) {
        return Err(inconsistent("stage time is outside supported years"));
    }
    Ok(time)
}
