import { prepared, detail, id, hash, instant, administration, history } from './deadline-unit.mjs';

export { instant, administration };
export const ids = id;
export const digest = hash;
export const clone = (value) => structuredClone(value);
export const notChecked = () => ({
  freshness: 'not_checked',
  checked_at: null,
  changed_dependencies: [],
  due_at: null,
});

// Digests are fixed transport fixtures, not evidence of cryptographic validity.
export function v2Prepared(action = 'register') {
  const value = prepared(action);
  value.author = { kind: 'user', id: value.actor_id, email: 'staff@example.test' };
  value.tracking = {
    policies: { profile: 'follow', source: 'undetermined', calendar: 'undetermined' },
    review: { state: 'accepted', reasons: [] },
    observations: {
      case_id: id(1),
      entries: [
        {
          role: 'profile',
          family: 'profile',
          id: id(2),
          revision: 1,
          case_id: id(1),
          hearing_id: null,
          parent_resolution: null,
          submission_digest: hash('b'),
          evidence_digest: hash('a'),
        },
      ],
    },
    administration: clone(value.calculation.material.administration),
  };
  value.receipt_version = {
    kind: 'v2',
    observations_digest: hash('9'),
    predecessor:
      action === 'register'
        ? null
        : {
            submission_digest: hash('e'),
            capture_digest: hash('d'),
          },
    cause: null,
  };
  if (['register', 'correct'].includes(action))
    value.command.change.tracking = clone(value.tracking.policies);
  return value;
}

export function v2Record(value = v2Prepared()) {
  const row = detail(value);
  row.receipt.version = clone(value.receipt_version);
  row.recorded_by = clone(value.author);
  row.tracking = clone(value.tracking);
  row.operational = notChecked();
  return row;
}

export function v1Record(value = prepared()) {
  const row = detail(value);
  row.receipt.version = { kind: 'v1' };
  row.recorded_by.kind = 'user';
  row.tracking = null;
  row.operational = notChecked();
  return row;
}

export function technicalRecord() {
  const row = v2Record(v2Prepared('correct'));
  row.receipt.action = 'reevaluate';
  row.receipt.operation_id = id(7);
  row.reason = 'Observed profile change';
  row.recorded_by = { kind: 'technical', service: 'deadline_reevaluator', policy_version: 1 };
  row.receipt.version.cause = {
    kind: 'source_event',
    job_id: id(0),
    event: {
      sequence: '9007199254740993',
      family: 'profile',
      source_id: id(2),
      revision: 2,
      case_id: id(1),
      hearing_id: null,
      operation_id: id(8),
    },
  };
  row.tracking.observations.entries[0].revision = 2;
  row.tracking.observations.entries[0].submission_digest = hash('8');
  row.tracking.observations.entries[0].evidence_digest = hash('7');
  row.tracking.review = {
    state: 'pending',
    reasons: [{ dependency: 'profile', reason: 'profile_changed' }],
  };
  return row;
}

export function timedPrepared() {
  const value = v2Prepared();
  const reference = { family: 'resolution', id: id(8), revision: 1 };
  value.definition.input.selection.source = { kind: 'known', value: reference };
  value.tracking.policies.source = 'fixed';
  value.command.change.definition = clone(value.definition);
  value.command.change.tracking = clone(value.tracking.policies);
  const source = {
    case_id: id(1),
    reference,
    values_digest: hash('1'),
    sources_digest: hash('2'),
    submission_digest: hash('3'),
    status: 'recorded',
    href: `/api/v1/cases/${id(1)}/resolutions/${id(8)}/revisions/1`,
  };
  value.calculation.material.source = source;
  value.calculation.material.source_head = clone(source);
  value.tracking.observations.entries.push({
    role: 'source',
    family: 'resolution',
    id: id(8),
    revision: 1,
    case_id: id(1),
    hearing_id: null,
    parent_resolution: null,
    submission_digest: hash('3'),
    evidence_digest: hash('4'),
  });
  const start = { unix_seconds: 1767225600, nanosecond: 0, offset_seconds: 0 };
  const due = { ...start, unix_seconds: 1767312000 };
  const at = {
    precision: 'second',
    year: 2026,
    month: 1,
    day: 1,
    hour: 0,
    minute: 0,
    second: 0,
    offset_seconds: 0,
  };
  const rule = { kind: 'elapsed_hours', quantity: 24 };
  value.calculation.result = {
    requirement: { kind: 'source_field', field: 'resolution_issued_at' },
    trigger_outcome: { kind: 'extracted', at },
    rule,
    arithmetic: {
      rule: clone(rule),
      anchor: clone(at),
      outcome: { kind: 'instant_candidate', instant: clone(due) },
      trace: [{ kind: 'elapsed_hours', start, quantity: 24, candidate: clone(due) }],
    },
    due_at: due,
    blocks: [],
  };
  return value;
}

export function summary(row = v2Record()) {
  return {
    id: row.id,
    case_id: row.case_id,
    revision: row.revision,
    title: row.definition.title,
    status: row.status,
    responsible: clone(row.responsible),
    attention_recorded: row.attention.status === 'recorded',
    receipt_kind: row.receipt.version.kind,
    review_state: row.tracking?.review.state ?? 'legacy_undeclared',
    calculation_due_at: clone(row.calculation.result.due_at),
    calculation_blocked: row.calculation.result.due_at === null,
    operational: clone(row.operational),
  };
}

export function historyRow(row = v2Record()) {
  return clone(history(row));
}
