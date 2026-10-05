use super::{inconsistent, port};
use application::{
    case_stages::{CaseStageEntry, StageDocumentFormat, StageFormatPolicy, StageSupportSnapshot},
    cases::CaseAdministrationSnapshot,
    precautionary_hearings::{
        PrecautionaryContext, PrecautionaryContextMaterial, PrecautionaryHearingCapture,
    },
    typed_participants::{DirectoryStatus, ParticipantDetail, ParticipantRevisionSnapshot},
    ApplicationError,
};
use domain::{
    case_administration::{CaseAdministrativeStatus, CaseRevision, CaseStageRevision},
    cases::CaseId,
    crypto::{ArchiveEntry, DocumentHasher, Sha256Digest},
    hearings::{HearingParticipantRef, HearingSupportRef},
    precautionary_hearings::PrecautionaryHearingValues,
};
use postgres::Transaction;

// Domain text limits count characters; four UTF-8 bytes per character bound transfer.
const ADMIN_BOUNDS: &str = "octet_length(a.title)<=800 AND octet_length(a.reference)<=400
 AND octet_length(a.administrative_status)<=6 AND octet_length(a.nuc)<=400
 AND octet_length(a.nuc_authority)<=800 AND octet_length(a.judicial_case_number)<=400
 AND octet_length(a.judicial_authority)<=800 AND cardinality(a.offenses) BETWEEN 1 AND 8
 AND NOT EXISTS(SELECT 1 FROM unnest(a.offenses) item WHERE octet_length(item)>480)
 AND COALESCE(octet_length(a.general_information),0)<=4000
 AND COALESCE(octet_length(a.complementary_identifiers),0)<=1200
 AND octet_length(a.changed_at)<=64 AND octet_length(a.changed_by_email) BETWEEN 1 AND 1280
 AND octet_length(a.values_digest)=32";

fn administration(
    tx: &mut Transaction<'_>,
    case: CaseId,
    revision: CaseRevision,
    hasher: &dyn DocumentHasher,
) -> Result<CaseAdministrationSnapshot, ApplicationError> {
    let row = tx.query_opt(&format!(
        "SELECT a.* FROM case_administration_revisions a WHERE a.case_id=$1 AND a.revision=$2 AND {ADMIN_BOUNDS}"
    ), &[&case.as_uuid(), &i64::from(revision.get())]).map_err(port)?
        .ok_or_else(|| inconsistent("administration source is absent or exceeds storage bounds"))?;
    crate::cases::values::decode(&row, hasher)
}

pub(super) fn current_context(
    tx: &mut Transaction<'_>,
    case: CaseId,
    hasher: &dyn DocumentHasher,
) -> Result<PrecautionaryContext, ApplicationError> {
    let admin = tx.query_opt("SELECT revision FROM case_administration_revisions WHERE case_id=$1 ORDER BY revision DESC LIMIT 1", &[&case.as_uuid()])
        .map_err(port)?.ok_or(ApplicationError::CaseStageProfileIncomplete)?;
    let admin = CaseRevision::new(u32::try_from(admin.get::<_, i64>(0)).map_err(inconsistent)?)
        .map_err(inconsistent)?;
    let stage = tx.query_opt("SELECT max(revision) FROM (SELECT revision FROM case_stage_revisions WHERE case_id=$1 UNION ALL SELECT stage_revision FROM case_initial_stage_registrations WHERE case_id=$1) s", &[&case.as_uuid()])
        .map_err(port)?.and_then(|row| row.get::<_, Option<i64>>(0)).ok_or(ApplicationError::CaseStageRequired)?;
    let stage = CaseStageRevision::new(u32::try_from(stage).map_err(inconsistent)?)
        .map_err(inconsistent)?;
    let context = exact_context(tx, case, admin, stage, hasher)?;
    if context.material().administration.values.status() != CaseAdministrativeStatus::Active {
        return Err(ApplicationError::CaseClosed);
    }
    Ok(context)
}

pub(super) fn exact_context(
    tx: &mut Transaction<'_>,
    case: CaseId,
    admin: CaseRevision,
    stage: CaseStageRevision,
    hasher: &dyn DocumentHasher,
) -> Result<PrecautionaryContext, ApplicationError> {
    let observed = administration(tx, case, admin, hasher)?;
    let keys = tx.query("SELECT administration_revision,FALSE AS initial FROM case_stage_revisions WHERE case_id=$1 AND revision=$2 UNION ALL SELECT administration_revision,TRUE FROM case_initial_stage_registrations WHERE case_id=$1 AND stage_revision=$2", &[&case.as_uuid(), &i64::from(stage.get())]).map_err(port)?;
    if keys.len() != 1 {
        return Err(inconsistent("exact stage source is absent or duplicated"));
    }
    let origin = CaseRevision::new(u32::try_from(keys[0].get::<_, i64>(0)).map_err(inconsistent)?)
        .map_err(inconsistent)?;
    let stage_administration = if origin == admin {
        observed.clone()
    } else {
        administration(tx, case, origin, hasher)?
    };
    let entry = if keys[0].get::<_, bool>(1) {
        let row = tx.query_opt(&format!("SELECT s.case_id,s.stage_revision,s.administration_revision,s.stage,a.values_digest AS administration_digest,a.changed_at AS recorded_at_text,a.changed_by AS recorded_by,a.changed_by_email AS recorded_by_email,(a.administrative_status='active' AND a.nuc IS NOT NULL) AS canonical FROM case_initial_stage_registrations s JOIN case_administration_revisions a ON a.case_id=s.case_id AND a.revision=s.administration_revision WHERE s.case_id=$1 AND s.stage_revision=$2 AND octet_length(s.stage)<=13 AND {ADMIN_BOUNDS}"), &[&case.as_uuid(), &i64::from(stage.get())]).map_err(port)?
            .ok_or_else(|| inconsistent("initial stage source exceeds storage bounds"))?;
        CaseStageEntry::Initial(crate::case_stages::decode::initial(&row)?)
    } else {
        let row = tx.query_opt(&format!("{} WHERE s.case_id=$1 AND s.revision=$2
 AND octet_length(s.change_kind)<=15 AND COALESCE(octet_length(s.from_stage),0)<=13 AND octet_length(s.stage)<=13
 AND octet_length(s.act_precision)<=7 AND COALESCE(octet_length(s.received_precision),0)<=7
 AND COALESCE(octet_length(s.reason),0)<=4000 AND COALESCE(octet_length(s.note),0)<=4000
 AND COALESCE(octet_length(s.receiving_court),0)<=800 AND COALESCE(octet_length(s.receipt_reference),0)<=800
 AND octet_length(s.support_name)<=128 AND octet_length(s.support_digest)=32
 AND octet_length(s.support_format)<=4 AND octet_length(s.support_policy)<=11
 AND COALESCE(octet_length(s.receipt_name),0)<=128 AND COALESCE(octet_length(s.receipt_digest),32)=32
 AND COALESCE(octet_length(s.receipt_format),0)<=4 AND COALESCE(octet_length(s.receipt_policy),0)<=11
 AND octet_length(s.recorded_by_email) BETWEEN 1 AND 1280 AND octet_length(s.values_digest)=32 AND {ADMIN_BOUNDS}", crate::case_stages::query::CHANGE_SELECT), &[&case.as_uuid(), &i64::from(stage.get())]).map_err(port)?
            .ok_or_else(|| inconsistent("changed stage source exceeds storage bounds"))?;
        let changed = crate::case_stages::decode::changed(&row, hasher)?;
        for captured in &changed.supports {
            let actual = document(
                tx,
                case,
                HearingSupportRef::new(captured.reference, captured.digest),
                captured.format,
            )?;
            if actual != *captured {
                return Err(inconsistent(
                    "stage support differs from its exact document",
                ));
            }
        }
        CaseStageEntry::Changed(Box::new(changed))
    };
    PrecautionaryContext::new(
        hasher,
        PrecautionaryContextMaterial {
            case_id: case,
            administration: observed,
            stage: entry,
            stage_administration,
        },
    )
}

pub(super) fn participants(
    tx: &mut Transaction<'_>,
    case: CaseId,
    values: &PrecautionaryHearingValues,
    previous: Option<&PrecautionaryHearingCapture>,
    fresh: bool,
    hasher: &dyn DocumentHasher,
) -> Result<Vec<ParticipantDetail>, ApplicationError> {
    let mut result = Vec::with_capacity(values.participants().len());
    for reference in values.participants() {
        let detail = exact_participant(tx, case, *reference, hasher)?;
        let retained = previous.and_then(|capture| {
            capture.review.sources.participants.iter().find(|old| {
                old.id() == reference.id() && old.revision_number() == reference.revision()
            })
        });
        if let Some(retained) = retained {
            if retained != &detail {
                return Err(inconsistent(
                    "retained participant differs from immutable source",
                ));
            }
        } else {
            let status = match &detail.revision {
                ParticipantRevisionSnapshot::Manual(value) => value.values.directory_status(),
                ParticipantRevisionSnapshot::Typed(value) => value.values.directory_status(),
            };
            if status != DirectoryStatus::Active {
                return Err(inconsistent("new participant selection was not active"));
            }
            if fresh {
                let head = tx.query_one("SELECT max(revision) FROM (SELECT revision FROM case_participant_revisions WHERE participant_id=$1 UNION ALL SELECT revision FROM case_participant_typed_revisions WHERE participant_id=$1) p", &[&reference.id().as_uuid()]).map_err(port)?;
                if head.get::<_, Option<i64>>(0) != Some(i64::from(reference.revision().get())) {
                    return Err(inconsistent("new participant selection is not current"));
                }
            }
        }
        result.push(detail);
    }
    Ok(result)
}

fn exact_participant(
    tx: &mut Transaction<'_>,
    case: CaseId,
    reference: HearingParticipantRef,
    hasher: &dyn DocumentHasher,
) -> Result<ParticipantDetail, ApplicationError> {
    let id = reference.id().as_uuid();
    let revision = i64::from(reference.revision().get());
    let keys = tx.query("SELECT FALSE AS typed FROM case_participant_revisions WHERE participant_id=$1 AND revision=$2 UNION ALL SELECT TRUE FROM case_participant_typed_revisions WHERE participant_id=$1 AND revision=$2", &[&id, &revision]).map_err(port)?;
    if keys.len() != 1 {
        return Err(inconsistent("participant source is absent or duplicated"));
    }
    if !keys[0].get::<_, bool>(0) {
        let row = tx.query_opt("SELECT p.case_id,r.* FROM case_participants p JOIN case_participant_revisions r ON r.participant_id=p.id WHERE p.case_id=$1 AND p.id=$2 AND r.revision=$3
 AND octet_length(r.display_name)<=800 AND octet_length(r.procedural_role)<=320
 AND COALESCE(octet_length(r.organization),0)<=800 AND COALESCE(octet_length(r.legal_status),0)<=640
 AND octet_length(r.directory_status)<=8 AND octet_length(r.values_digest)=32
 AND octet_length(r.changed_at)<=64 AND octet_length(r.changed_by_email) BETWEEN 1 AND 1280", &[&case.as_uuid(), &id, &revision]).map_err(port)?
            .ok_or_else(|| inconsistent("manual participant is foreign or exceeds storage bounds"))?;
        return crate::participant_postgres::storage::decode(&row, hasher).map(Into::into);
    }
    let row = tx.query_opt("SELECT p.case_id,r.*,e.statement_digest FROM case_participants p JOIN case_participant_typed_revisions r ON r.participant_id=p.id LEFT JOIN participant_credential_evidence e ON e.participant_id=r.participant_id AND e.revision=r.credential_origin_revision WHERE p.case_id=$1 AND p.id=$2 AND r.revision=$3
 AND octet_length(r.values_canonical)<=8380 AND octet_length(r.values_view::text)<=65536
 AND octet_length(r.values_digest)=32 AND octet_length(r.submission_digest)=32
 AND octet_length(r.changed_at)<=64 AND octet_length(r.changed_by_email) BETWEEN 1 AND 1280
 AND octet_length(r.role_kind)<=64 AND octet_length(r.directory_status)<=8
 AND COALESCE(octet_length(r.organization),0)<=800 AND COALESCE(octet_length(e.statement_digest),32)=32", &[&case.as_uuid(), &id, &revision]).map_err(port)?
        .ok_or_else(|| inconsistent("typed participant is foreign or exceeds storage bounds"))?;
    let snapshot = crate::typed_participant_postgres::storage::decode_typed(&row, hasher)?;
    let bound = snapshot.values.subject();
    let row = tx.query_opt("SELECT s.case_id,r.* FROM case_subjects s JOIN case_subject_revisions r ON r.subject_id=s.id WHERE s.case_id=$1 AND s.id=$2 AND r.revision=$3
 AND octet_length(r.values_canonical)<=5676 AND octet_length(r.values_view::text)<=65536
 AND octet_length(r.values_digest)=32 AND octet_length(r.identity_document_digest)=32
 AND octet_length(r.changed_at)<=64 AND octet_length(r.changed_by_email) BETWEEN 1 AND 1280
 AND octet_length(r.subject_kind)<=32 AND octet_length(r.display_name)<=800
 AND COALESCE(octet_length(r.declared_identifier),0)<=800 AND octet_length(r.identity_document_locator)<=2000", &[&case.as_uuid(), &bound.id.as_uuid(), &i64::from(bound.revision.get())]).map_err(port)?
        .ok_or_else(|| inconsistent("bound subject is foreign or exceeds storage bounds"))?;
    let subject = crate::typed_participant_postgres::storage::decode_subject(&row, hasher)?;
    if subject.values_digest != bound.values_digest
        || !snapshot
            .values
            .kind()
            .accepts_subject(subject.values.kind())
    {
        return Err(inconsistent(
            "typed participant differs from its exact bound identity",
        ));
    }
    Ok(ParticipantDetail {
        revision: ParticipantRevisionSnapshot::Typed(Box::new(snapshot)),
        bound_subject: Some(subject),
    })
}

pub(super) fn support(
    tx: &mut Transaction<'_>,
    case: CaseId,
    values: &PrecautionaryHearingValues,
    format: StageDocumentFormat,
) -> Result<StageSupportSnapshot, ApplicationError> {
    document(tx, case, values.scheduling_basis().support(), format)
}

fn document(
    tx: &mut Transaction<'_>,
    case: CaseId,
    selected: HearingSupportRef,
    format: StageDocumentFormat,
) -> Result<StageSupportSnapshot, ApplicationError> {
    let reference = selected.reference();
    let row = tx.query_opt("SELECT name,digest FROM documents WHERE case_id=$1 AND id=$2 AND version=$3 AND octet_length(name) BETWEEN 1 AND 128 AND octet_length(digest)=32", &[&case.as_uuid(), &reference.id.as_uuid(), &i64::from(reference.version.get())]).map_err(port)?
        .ok_or_else(|| inconsistent("exact documentary support is absent or exceeds storage bounds"))?;
    let bytes: Vec<u8> = row.get("digest");
    let digest = Sha256Digest::from_array(
        bytes
            .try_into()
            .map_err(|_| inconsistent("invalid source digest length"))?,
    );
    if digest != selected.digest() {
        return Err(inconsistent("exact documentary digest differs"));
    }
    let name: String = row.get("name");
    ArchiveEntry::new(name.clone(), Vec::new()).map_err(inconsistent)?;
    Ok(StageSupportSnapshot {
        reference,
        digest,
        name,
        format,
        policy: StageFormatPolicy::PdfDocxV1,
    })
}
