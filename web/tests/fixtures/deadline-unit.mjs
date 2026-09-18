export const id = (n) => `00000000-0000-0000-0000-${String(n).padStart(12, '0')}`;
export const hash = (n = 'a') => n.repeat(64);
export const known = (value) => ({ kind: 'known', value });
export const instant = () => ({
  unix_seconds: 1767225600,
  nanosecond: 123456789,
  offset_seconds: -21600,
});
export function definition() {
  return {
    title: 'Respuesta declarada',
    profile: { id: id(2), revision: 1 },
    responsible_id: id(4),
    input: {
      selection: {
        case_id: id(1),
        source: { kind: 'unknown', reason: 'Fuente pendiente' },
        qualification: null,
      },
      calendar: null,
      ordered_quantity: null,
      qualification: {
        statement: 'Aplicabilidad declarada',
        locator: 'Acto, pagina 1',
        scope_applies: known(true),
        unresolved_incident: known(false),
        conditions: [],
      },
    },
  };
}
export function prepared(action = 'register') {
  const def = definition();
  const change = { action, expected_revision: action === 'register' ? 0 : 1 };
  if (['register', 'correct'].includes(action)) change.definition = structuredClone(def);
  if (action !== 'register') change.reason = 'Actualizacion declarada';
  if (action === 'set_attention') change.attention = { status: 'pending' };
  return {
    case_id: id(1),
    actor_id: id(4),
    command: { operation_id: id(5), deadline_id: id(6), change },
    result_revision: change.expected_revision + 1,
    definition: def,
    calculation: {
      profile: {
        id: id(2),
        revision: 1,
        algorithm: 'v1',
        title: 'Horas declaradas',
        scope: { kind: 'case', case_id: id(1) },
        status: 'published',
        definition_digest: hash(),
        submission_digest: hash('b'),
        href: `/api/v1/cases/${id(1)}/deadline-profiles/${id(2)}/revisions/1`,
      },
      material: {
        case_id: id(1),
        administration: {
          kind: 'unrevised',
          title: 'Expediente',
          reference: null,
          status: 'active',
        },
        source: null,
        source_head: null,
        calendar: null,
        calendar_head: null,
      },
      result: {
        requirement: { kind: 'source_field', field: 'resolution_issued_at' },
        trigger_outcome: { kind: 'blocked', block: { kind: 'unknown_source' } },
        rule: { kind: 'elapsed_hours', quantity: 24 },
        arithmetic: null,
        due_at: null,
        blocks: [{ kind: 'trigger', block: { kind: 'unknown_source' } }],
      },
    },
    responsible: { id: id(4), email: 'staff@example.test', role: 'owner' },
    attention: { status: 'pending' },
    status: action === 'retire' ? 'retired' : 'active',
    review_digest: hash('c'),
    capture_digest: hash('d'),
    submission_digest: hash('e'),
  };
}
export function detail(value = prepared()) {
  return {
    id: value.command.deadline_id,
    case_id: value.case_id,
    revision: value.result_revision,
    definition: structuredClone(value.definition),
    calculation: structuredClone(value.calculation),
    responsible: structuredClone(value.responsible),
    attention: structuredClone(value.attention),
    status: value.status,
    reason: value.command.change.reason ?? null,
    receipt: {
      operation_id: value.command.operation_id,
      action: value.command.change.action,
      expected_revision: value.command.change.expected_revision,
      review_digest: value.review_digest,
      capture_digest: value.capture_digest,
      submission_digest: value.submission_digest,
    },
    recorded_at: instant(),
    recorded_by: { id: value.actor_id, email: 'staff@example.test' },
  };
}
export function administration(revision = 1) {
  return {
    kind: 'recorded',
    case_id: id(1),
    revision,
    title: 'Expediente actualizado',
    reference: null,
    status: 'active',
    values_digest: hash('f'),
    changed_at: instant(),
    changed_by: { id: id(4), email: 'staff@example.test' },
  };
}
export function summary(value = detail()) {
  return {
    id: value.id,
    case_id: value.case_id,
    revision: value.revision,
    title: value.definition.title,
    status: value.status,
    responsible: value.responsible,
    attention_recorded: value.attention.status === 'recorded',
    due_at: value.calculation.result.due_at,
    blocked: value.calculation.result.due_at === null,
  };
}
export function history(value = detail()) {
  const { id, case_id, revision, status, reason, receipt, recorded_at, recorded_by } = value;
  return {
    id,
    case_id,
    revision,
    status,
    reason,
    receipt,
    recorded_at,
    recorded_by,
    state_digest: receipt.review_digest,
  };
}

export function profile(caseId = id(1)) {
  const scope = { kind: 'case', case_id: caseId };
  const rule = { kind: 'elapsed_hours', quantity: 24 };
  const reference = {
    id: id(20),
    title: 'Fuente de prueba',
    issuer: 'Institucion',
    official_url: 'https://example.test/fixture',
    published_on: null,
    consulted_on: '2026-01-01',
    locator: 'Articulo 1',
  };
  return {
    collection: { kind: 'case', case_id: caseId },
    id: id(2),
    revision: 1,
    status: 'published',
    algorithm: 'v1',
    definition_digest: hash(),
    scope,
    reason: null,
    receipt: {
      operation_id: id(21),
      action: 'publish',
      expected_revision: 0,
      submission_digest: hash('b'),
    },
    recorded_at: '2026-01-01T00:00:00Z',
    recorded_by: { id: id(4), email: 'staff@example.test' },
    definition: {
      title: 'Horas declaradas',
      description: 'Perfil de prueba',
      scope,
      references: [reference],
      trigger: { kind: 'source_field', field: 'resolution_issued_at' },
      template: { kind: 'fixed', rule },
      completion: { kind: 'arithmetic_instant' },
      conditions: [{ id: id(22), statement: 'Supuesto declarado', reference_ids: [id(20)] }],
      examples: [
        {
          id: id(23),
          anchor: {
            precision: 'second',
            year: 2026,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
            offset_seconds: 0,
          },
          ordered_quantity: null,
          calendar: null,
          expected: {
            kind: 'arithmetic',
            outcome: {
              kind: 'instant_candidate',
              instant: { unix_seconds: 1767312000, nanosecond: 0, offset_seconds: 0 },
            },
          },
          reference_ids: [id(20)],
          locator: 'Ejemplo 1',
        },
      ],
    },
  };
}
