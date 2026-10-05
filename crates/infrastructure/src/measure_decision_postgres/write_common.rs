use super::{anchors, inconsistent, member_fields::MemberFields, port};
use application::{
    case_stages::StageSupportSnapshot, identity::Principal,
    precautionary_hearings::PrecautionaryContext, precautionary_measures::MeasureDecisionCommand,
    ApplicationError,
};
use domain::{
    cases::CaseId,
    crypto::{DocumentHasher, Sha256Digest},
    precautionary_measures::MeasureSupervision,
};
use postgres::Transaction;

pub(super) struct DecisionFields<'a> {
    pub command: &'a MeasureDecisionCommand,
    pub case: CaseId,
    pub context: &'a PrecautionaryContext,
    pub support: &'a StageSupportSnapshot,
    pub actor: &'a Principal,
    pub at: time::OffsetDateTime,
    pub submission_digest: Sha256Digest,
    pub review_digest: Sha256Digest,
    pub decision_digest: Sha256Digest,
    pub group_digest: Sha256Digest,
    pub family: &'static str,
    pub marker: String,
}
pub(super) fn decision(
    tx: &mut Transaction<'_>,
    fields: DecisionFields<'_>,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let command = fields.command;
    let entry = crate::audit_postgres::append_transaction(
        tx,
        &fields.actor.email,
        "measure_decision.recorded",
        &fields.marker,
        fields.at,
    )?;
    let sequence = i64::try_from(entry.event.sequence)
        .map_err(|_| inconsistent("measure audit sequence exceeds storage range"))?;
    tx.execute(
        "INSERT INTO case_measure_operations(operation_id,case_id,family,owner_digest,audit_sequence)
         VALUES($1,$2,$5,$3,$4)",
        &[&command.operation_id.as_uuid(), &fields.case.as_uuid(),
            &fields.group_digest.as_bytes().as_slice(), &sequence, &fields.family],
    ).map_err(port)?;
    let values = command.values.canonical_bytes();
    let values_view = crate::measure_decision_codec::decision_view(&command.values);
    let values_digest = hasher.hash_bytes(&values);
    let outcome = command.outcome.canonical_bytes();
    let outcome_view = crate::measure_decision_codec::outcome_view(&command.outcome);
    let outcome_digest = hasher.hash_bytes(&outcome);
    let context = fields.context;
    let context_digest = context.digest(hasher);
    let support = fields.support;
    let anchor = anchors::columns(&command.anchor)?;
    tx.execute(
        "INSERT INTO case_measure_decisions(decision_id,operation_id,case_id,
            values_canonical,values_view,values_digest,outcome_canonical,outcome_view,outcome_digest,
            observed_administration_revision,observed_stage_revision,observed_context_digest,
            support_format,support_policy,recorded_by,recorded_by_email,recorded_by_role,
            recorded_at_seconds,recorded_at_nanoseconds,submission_digest,review_digest,
            decision_digest,group_digest,anchor_kind,anchor_hearing_id,anchor_revision,
            anchor_values_digest,anchor_submission_digest,anchor_precautionary_hearing_id,anchor_capture_digest)
         VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27,$28,$29,$30)",
        &[
            &command.decision_id.as_uuid(), &command.operation_id.as_uuid(), &fields.case.as_uuid(),
            &values, &values_view, &values_digest.as_bytes().as_slice(),
            &outcome, &outcome_view, &outcome_digest.as_bytes().as_slice(),
            &i64::from(context.material().administration.revision.get()),
            &i64::from(context.material().stage.stage_revision().get()),
            &context_digest.as_bytes().as_slice(), &support.format.as_str(), &support.policy.as_str(),
            &fields.actor.id.as_uuid(), &fields.actor.email, &fields.actor.role.as_str(),
            &fields.at.unix_timestamp(), &(fields.at.nanosecond() as i32),
            &fields.submission_digest.as_bytes().as_slice(), &fields.review_digest.as_bytes().as_slice(),
            &fields.decision_digest.as_bytes().as_slice(), &fields.group_digest.as_bytes().as_slice(),
            &anchor.kind, &anchor.hearing_id, &anchor.revision,
            &anchor.values_digest, &anchor.submission_digest, &anchor.precautionary_hearing_id, &anchor.capture_digest,
        ],
    ).map_err(port)?;
    Ok(())
}
pub(super) fn member(
    tx: &mut Transaction<'_>,
    fields: MemberFields<'_>,
    is_root: bool,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    let subject = fields.values.subject();
    let (supervisor_id, supervisor_revision) = match fields.values.supervision() {
        MeasureSupervision::Known { participant, .. } => (
            Some(participant.id().as_uuid()),
            Some(i64::from(participant.revision().get())),
        ),
        MeasureSupervision::Unknown { .. } => (None, None),
    };
    let canonical = fields.values.canonical_bytes();
    let projection = crate::measure_decision_codec::measure_view(fields.values);
    let digest = hasher.hash_bytes(&canonical);
    if is_root {
        tx.execute(
            "INSERT INTO case_measures(id,case_id,root_operation) VALUES($1,$2,$3)",
            &[
                &fields.id.as_uuid(),
                &fields.case.as_uuid(),
                &fields.root_operation.as_uuid(),
            ],
        )
        .map_err(port)?;
    }
    tx.execute(
        "INSERT INTO case_measure_revisions(measure_id,revision,case_id,owner_operation,
                family,action,values_canonical,values_view,values_digest,capture_digest,
                subject_id,subject_revision,subject_values_digest,supervisor_id,supervisor_revision)
             VALUES($1,$2,$3,$4,$15,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
        &[
            &fields.id.as_uuid(),
            &i64::from(fields.revision.get()),
            &fields.case.as_uuid(),
            &fields.operation.as_uuid(),
            &super::decode::action_name(fields.action),
            &canonical,
            &projection,
            &digest.as_bytes().as_slice(),
            &fields.digest.as_bytes().as_slice(),
            &subject.id.as_uuid(),
            &i64::from(subject.revision.get()),
            &subject.values_digest.as_bytes().as_slice(),
            &supervisor_id,
            &supervisor_revision,
            &fields.family,
        ],
    )
    .map_err(port)?;
    Ok(())
}
