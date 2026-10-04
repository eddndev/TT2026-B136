import { resultPrepared, resultRecord } from './hearing-results.mjs';
import { profile, administration } from './deadline-unit.mjs';
import { v2Prepared, v2Record, ids, digest, clone } from './deadline-v2-unit.mjs';

export { ids, digest, clone };
export const principal = () => ({ id: ids(4), email: 'staff@example.test', role: 'owner' });
const scoped = (value, caseId) => JSON.parse(JSON.stringify(value).replaceAll(ids(1), caseId));

// Fixed digests exercise transport consistency, not cryptographic validity.
export function derivedReady() {
  const result = resultPrepared();
  result.actor_id = principal().id;
  result.values.event_time = { precision: 'instant', at: '2026-09-01T10:00:00-06:00' };
  result.values.agreements = [{ id: ids(70), text: 'Declared agreement' }];
  result.command.change.values = clone(result.values);
  const deadline = scoped(v2Prepared(), result.case_id);
  const selected = profile(result.case_id);
  selected.definition.trigger = {
    kind: 'qualified',
    purpose: 'hearing_end',
    family: 'hearing_result',
  };
  const definition = deadline.definition;
  definition.input.selection.source = {
    kind: 'known',
    value: {
      family: 'hearing_result',
      hearing_id: result.command.hearing_id,
      result_id: result.command.result_id,
      revision: 1,
      agreement_id: ids(70),
    },
  };
  definition.input.qualification.conditions = [
    { id: ids(22), applies: { kind: 'known', value: true }, locator: 'Declared condition' },
  ];
  deadline.command.change.definition = clone(definition);
  deadline.command.change.tracking = {
    profile: 'fixed',
    source: 'follow',
    calendar: 'undetermined',
  };
  const block = { kind: 'missing_qualification', purpose: 'hearing_end' };
  return {
    state: 'ready',
    command: { case_id: result.case_id, result: clone(result.command), deadline: deadline.command },
    result,
    deadline: {
      definition,
      tracking: clone(deadline.command.change.tracking),
      responsible: clone(deadline.responsible),
      profile: selected,
      profile_head: clone(selected),
      calendar: null,
      calendar_head: null,
      result: {
        requirement: clone(selected.definition.trigger),
        trigger_outcome: { kind: 'blocked', block: clone(block) },
        rule: clone(selected.definition.template.rule),
        arithmetic: null,
        due_at: null,
        blocks: [{ kind: 'trigger', block }],
      },
    },
    review_digest: digest('6'),
  };
}

export function derivedRecord(ready = derivedReady()) {
  const c = ready.command;
  const result = resultRecord(ready.result);
  result.recorded_by = { id: principal().id, email: principal().email };
  result.recorded_at = '2026-09-16T12:00:00.123456789Z';
  const draft = scoped(v2Prepared(), c.case_id);
  draft.command = clone(c.deadline);
  draft.definition = clone(ready.deadline.definition);
  draft.author = { kind: 'user', ...result.recorded_by };
  draft.responsible = clone(ready.deadline.responsible);
  const p = ready.deadline.profile;
  draft.calculation.profile = {
    id: p.id,
    revision: p.revision,
    algorithm: p.algorithm,
    title: p.definition.title,
    scope: clone(p.scope),
    status: p.status,
    definition_digest: p.definition_digest,
    submission_digest: p.receipt.submission_digest,
    href: `/api/v1/cases/${c.case_id}/deadline-profiles/${p.id}/revisions/${p.revision}`,
  };
  const source = {
    case_id: c.case_id,
    reference: clone(c.deadline.change.definition.input.selection.source.value),
    values_digest: result.values_digest,
    sources_digest: null,
    submission_digest: result.receipt.submission_digest,
    status: result.status,
    href: `/api/v1/cases/${c.case_id}/hearings/${result.hearing_id}/results/${result.id}/revisions/1`,
  };
  const head = clone(source);
  head.reference.agreement_id = null;
  const admin = scoped(administration(), c.case_id);
  admin.values_digest = result.recorded_administration_digest;
  admin.changed_at.offset_seconds = 0;
  draft.calculation.material = {
    case_id: c.case_id,
    administration: admin,
    source,
    source_head: head,
    calendar: null,
    calendar_head: null,
  };
  draft.calculation.result = clone(ready.deadline.result);
  draft.tracking.policies = clone(ready.deadline.tracking);
  draft.tracking.administration = clone(admin);
  draft.tracking.observations.entries.push({
    role: 'source',
    family: 'hearing_result',
    id: result.id,
    revision: 1,
    case_id: c.case_id,
    hearing_id: result.hearing_id,
    parent_resolution: null,
    submission_digest: result.receipt.submission_digest,
    evidence_digest: digest('8'),
  });
  const deadline = v2Record(draft);
  deadline.recorded_at = {
    unix_seconds: Date.parse('2026-09-16T12:00:00Z') / 1000,
    nanosecond: 123456789,
    offset_seconds: 0,
  };
  const origin = {
    case_id: c.case_id,
    hearing_id: result.hearing_id,
    result_id: result.id,
    result_revision: 1,
    result_operation_id: c.result.operation_id,
    deadline_id: deadline.id,
    deadline_revision: 1,
    deadline_operation_id: c.deadline.operation_id,
    recorded_by: principal(),
    review_digest: ready.review_digest,
    capture_digest: digest('7'),
    source_event: {
      sequence: '9007199254740993',
      family: 'hearing_result',
      source_id: result.id,
      revision: 1,
      case_id: c.case_id,
      hearing_id: result.hearing_id,
      operation_id: c.result.operation_id,
    },
  };
  return {
    case_id: c.case_id,
    command: clone(c),
    result,
    deadline,
    origin,
    review_digest: origin.review_digest,
    capture_digest: origin.capture_digest,
  };
}
