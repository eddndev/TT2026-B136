import {
  precautionaryHearingOperation,
  precautionaryCaseId,
  clone,
} from './precautionary-hearing-unit.mjs';

export { clone };
export const measureCaseId = precautionaryCaseId;
export const measureId = 'a1000000-0000-4000-8000-000000000001';
export const otherMeasureId = 'a1000000-0000-4000-8000-000000000002';
export const foreignMeasureId = 'ffffffff-ffff-4fff-8fff-ffffffffffff';
const operation = (n) => `a2000000-0000-4000-8000-${String(n).padStart(12, '0')}`;
const decision = (n) => `a3000000-0000-4000-8000-${String(n).padStart(12, '0')}`;
const hash = (n) => String(n).repeat(64);
const base = () => precautionaryHearingOperation({ revision: 1 }).capture.review;

function subject() {
  const support = base().resolved_values.scheduling_basis.support;
  return {
    case_id: measureCaseId,
    id: 'a4000000-0000-4000-8000-000000000001',
    revision: 1,
    values: {
      kind: 'natural_person',
      name: { state: 'known', value: 'Persona declarada' },
      curp: { state: 'unknown', reason: 'No consta en el soporte' },
      identity_support: { ...support, locator: 'Pagina 1' },
    },
    values_digest: hash('b'),
    changed_at: '2026-01-01T00:00:00Z',
    changed_by: { id: base().actor.id, email: base().actor.email },
  };
}

function material() {
  const source = subject();
  return {
    values: {
      subject: { id: source.id, revision: source.revision, values_digest: source.values_digest },
      kind: 'periodic_appearance',
      conditions: 'Comparecer segun la resolucion declarada',
      validity: {
        start: { precision: 'date', year: 2026, month: 1, day: 2, offset_seconds: null },
        statement: 'Vigencia declarada sin termino conocido',
        end: null,
      },
      supervision: { kind: 'unknown', reason: 'No consta la autoridad supervisora' },
    },
    sources: { subject: source, supervisor: null },
    projection: {
      subject: {
        case_id: measureCaseId,
        id: source.id,
        revision: source.revision,
        kind: source.values.kind,
        display_name: source.values.name.value,
      },
      supervisor: null,
    },
  };
}

function group(detail, previous = null) {
  const { capture, owner } = detail.record,
    { result } = capture;
  const common = base();
  const values = {
    authority: 'Juzgado declarado',
    declared_at: { precision: 'unknown', reason: 'No consta la hora de decision' },
    justification: 'Decision declarada en el soporte',
    support: common.resolved_values.scheduling_basis.support,
    locator: 'Pagina 1',
  };
  const command = {
    case_id: measureCaseId,
    operation_id: owner.operation_id,
    decision_id: owner.decision_id,
    context: common.observed_context.expectation,
    values,
    anchor: null,
    outcome: {
      kind: 'changes',
      effects: [
        previous
          ? { action: 'revoke', previous: previous.reference }
          : { action: 'impose', proposal: { id: result.id, values: result.values } },
      ],
    },
  };
  const review = {
    case_id: measureCaseId,
    actor: common.actor,
    command,
    material: {
      context: common.observed_context,
      support: common.sources.support,
      anchor: null,
      predecessors: previous ? [clone(previous.record)] : [],
      result_sources: [{ id: result.id, sources: clone(result.sources) }],
    },
    results: [clone(result)],
    submission_digest: hash('c'),
    review_digest: hash('d'),
  };
  const decisionCapture = {
    case_id: measureCaseId,
    operation_id: owner.operation_id,
    decision_id: owner.decision_id,
    actor: common.actor,
    context: common.observed_context,
    values,
    support: common.sources.support,
    anchor: null,
    recorded_at: capture.recorded_at,
    capture_digest: capture.decision_digest,
  };
  return {
    origin: {
      case_id: measureCaseId,
      ...owner,
      submission_digest: review.submission_digest,
      review_digest: review.review_digest,
      decision_digest: capture.decision_digest,
    },
    capture: {
      family: detail.family === 'm1' ? 'g1' : 'g2',
      review,
      decision: decisionCapture,
      measures: [clone(capture)],
      substitutions: [],
      recorded_at: capture.recorded_at,
      capture_digest: owner.group_digest,
    },
  };
}

function judicial(family, id) {
  const revision = family === 'm1' ? 1 : 2;
  const origin = { operation_id: operation(1), decision_id: decision(1) };
  const root = { kind: 'judicial', origin: clone(origin) };
  const prior = family === 'm2' ? judicial('m1', id) : null;
  const owner = {
    operation_id: operation(revision),
    decision_id: decision(revision),
    group_digest: hash(revision + 3),
  };
  const reference = { id, revision, capture_digest: hash(revision) };
  const result = {
    id,
    revision,
    ...(family === 'm1' ? { origin } : { record_root: root, judicial_origin: origin }),
    effect_key: id,
    action: prior ? 'revoke' : 'impose',
    previous: prior?.reference || null,
    ...material(),
  };
  const capture = {
    family,
    case_id: measureCaseId,
    result,
    operation_id: owner.operation_id,
    decision_id: owner.decision_id,
    decision_digest: hash('e'),
    actor: base().actor,
    recorded_at: `2026-01-0${revision + 1}T12:00:00Z`,
    capture_digest: reference.capture_digest,
  };
  const detail = {
    case_id: measureCaseId,
    reference,
    family,
    validity: 'valid',
    last_action: result.action,
    record_root: root,
    judicial_origin: origin,
    last_judicial: { owner, reference },
    record: { family, owner, capture },
    record_history: prior
      ? clone(prior.record_history)
      : {
          records: { judicial: { groups: [] }, administrative: [] },
          decisions: [],
        },
  };
  const entry = group(detail, prior);
  if (family === 'm1') detail.record_history.records.judicial.groups.push(entry);
  else detail.record_history.decisions.push(entry);
  return detail;
}

export function measureRecord({ family = 'm1', id = measureId, validity = 'valid' } = {}) {
  if (family !== 'c1') return judicial(family, id);
  const prior = judicial('m1', id),
    common = base();
  const reference = { id, revision: 2, capture_digest: hash('6') };
  const owner = { operation_id: operation(3), capture_digest: hash('7') };
  const result = {
    id,
    revision: 2,
    previous: prior.reference,
    record_root: prior.record_root,
    judicial_origin: prior.judicial_origin,
    last_judicial: prior.last_judicial,
    last_action: prior.last_action,
    validity,
    ...material(),
  };
  const capture = {
    family,
    case_id: measureCaseId,
    operation_id: owner.operation_id,
    result,
    actor: common.actor,
    context: common.observed_context,
    support: common.sources.support,
    review_digest: hash('8'),
    recorded_at: '2026-01-04T12:00:00Z',
    capture_digest: reference.capture_digest,
  };
  const command = {
    case_id: measureCaseId,
    operation_id: owner.operation_id,
    target: prior.reference,
    context: common.observed_context.expectation,
    reason: 'Rectificacion declarada',
    action:
      validity === 'valid'
        ? {
            kind: 'correct',
            values: {
              conditions: result.values.conditions,
              validity: result.values.validity,
              supervision_text: result.values.supervision.reason,
            },
          }
        : { kind: 'entered_in_error' },
  };
  const review = {
    case_id: measureCaseId,
    actor: common.actor,
    command,
    context: common.observed_context,
    support: common.sources.support,
    result: clone(result),
    replacement: null,
    submission_digest: hash('9'),
    review_digest: capture.review_digest,
  };
  const history = clone(prior.record_history);
  history.records.administrative.push({
    origin: {
      case_id: measureCaseId,
      ...owner,
      submission_digest: review.submission_digest,
      review_digest: review.review_digest,
    },
    capture: {
      family: 'a1',
      review,
      records: [clone(capture)],
      replacement_link: null,
      recorded_at: capture.recorded_at,
      capture_digest: owner.capture_digest,
    },
  });
  return {
    case_id: measureCaseId,
    reference,
    family,
    validity,
    last_action: result.last_action,
    record_root: prior.record_root,
    judicial_origin: prior.judicial_origin,
    last_judicial: prior.last_judicial,
    record: { family, owner, capture },
    record_history: history,
  };
}

export function measurePage() {
  return {
    case_id: measureCaseId,
    items: [
      measureRecord(),
      measureRecord({ id: otherMeasureId, family: 'c1', validity: 'entered_in_error' }),
    ],
    has_more: false,
    next_after_id: null,
  };
}
