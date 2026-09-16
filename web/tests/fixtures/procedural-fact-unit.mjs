export const factCaseId = '10000000-0000-4000-8000-000000000001';
export const factActorId = '20000000-0000-4000-8000-000000000002';
export const resolutionId = '30000000-0000-4000-8000-000000000003';
export const notificationId = '40000000-0000-4000-8000-000000000004';
export const factOperationId = '50000000-0000-4000-8000-000000000005';
export const factDigest = 'a'.repeat(64);
const known = (kind) => ({ kind: 'known', value: { kind } });
export function factValuesFixture(family = 'resolution') {
  const common = {
    subtype: null,
    summary: 'Declaracion conservada',
    provenance: { kind: 'operator_note', note: 'Relato de la operadora' },
  };
  if (family === 'resolution')
    return {
      ...common,
      class: known('order'),
      issuer: { kind: 'known', value: 'Organo declarado' },
      issued_at: { precision: 'unknown' },
    };
  return {
    ...common,
    resolution: { id: resolutionId, revision: 1 },
    character: known('personal'),
    medium: known('in_person'),
    context: known('outside_hearing'),
    outcome: known('practiced'),
    practiced_at: { precision: 'unknown' },
    received_at: null,
    stated_effect: null,
    intended_recipient: { kind: 'unknown', reason: 'Sin identificar' },
    actual_receiver: { kind: 'unknown', reason: 'Sin identificar' },
    representation: { kind: 'not_recorded', reason: 'No se declaro representacion' },
  };
}
export function factCommandFixture(family = 'resolution', action = 'record') {
  return {
    family,
    id: family === 'resolution' ? resolutionId : notificationId,
    operation_id: factOperationId,
    ...(family === 'notification' ? { resolution_id: resolutionId } : {}),
    change: {
      action,
      expected_revision: action === 'record' ? 0 : 1,
      ...(action === 'record' ? {} : { reason: 'Precision declarada' }),
      ...(action === 'withdraw' ? {} : { values: factValuesFixture(family) }),
    },
  };
}
export function factPrepared(command = factCommandFixture()) {
  const values = structuredClone(command.change.values || factValuesFixture(command.family));
  const sources = { resolution: null, participants: [], hearing_results: [], direct_supports: [] };
  if (command.family === 'notification')
    sources.resolution = {
      case_id: factCaseId,
      id: command.resolution_id,
      revision: values.resolution.revision,
      values_digest: factDigest,
      submission_digest: 'b'.repeat(64),
      status: 'withdrawn',
      class: { kind: 'known', value: { kind: 'order' } },
      issuer: { kind: 'unknown', reason: 'No identificado' },
      issued_at: { precision: 'unknown' },
      summary: 'Resolucion historica',
    };
  return {
    case_id: factCaseId,
    actor_id: factActorId,
    command: structuredClone(command),
    result_revision: command.change.expected_revision + 1,
    values,
    values_digest: factDigest,
    sources,
    sources_digest: 'c'.repeat(64),
    submission_digest: 'd'.repeat(64),
    observed_administration: {
      kind: 'unrevised',
      title: 'Expediente anterior',
      reference: null,
      status: 'active',
    },
  };
}
export function factRecord(prepared = factPrepared()) {
  const command = prepared.command;
  return {
    case_id: prepared.case_id,
    family: command.family,
    id: command.id,
    ...(command.family === 'notification' ? { resolution_id: command.resolution_id } : {}),
    revision: prepared.result_revision,
    status: command.change.action === 'withdraw' ? 'withdrawn' : 'recorded',
    reason: command.change.reason || null,
    values: structuredClone(prepared.values),
    values_digest: prepared.values_digest,
    sources: structuredClone(prepared.sources),
    recorded_administration: structuredClone(prepared.observed_administration),
    recorded_by: { id: prepared.actor_id, email: 'captured@example.test' },
    recorded_at: '2026-09-16T12:00:00Z',
    receipt: {
      operation_id: command.operation_id,
      action: command.change.action,
      expected_revision: command.change.expected_revision,
      sources_digest: prepared.sources_digest,
      submission_digest: prepared.submission_digest,
    },
  };
}
export function factRow(record = factRecord()) {
  const { case_id, family, id, revision, status } = record;
  return family === 'resolution'
    ? {
        case_id,
        family,
        id,
        revision,
        status,
        class: record.values.class,
        issued_at: record.values.issued_at,
      }
    : {
        case_id,
        family,
        id,
        revision,
        status,
        resolution_id: record.resolution_id,
        resolution: record.values.resolution,
        outcome: record.values.outcome,
        practiced_at: record.values.practiced_at,
      };
}
export function factHistoryRow(record = factRecord()) {
  const result = structuredClone(record);
  delete result.values;
  delete result.sources;
  return result;
}
export function richFactPrepared() {
  const command = factCommandFixture('notification');
  const person = { kind: 'participant', id: '60000000-0000-4000-8000-000000000006', revision: 2 };
  const support = {
    document_id: '70000000-0000-4000-8000-000000000007',
    version: 1,
    digest: factDigest,
    locator: 'Pagina 1',
  };
  const hearing = {
    hearing_id: '80000000-0000-4000-8000-000000000008',
    result_id: '90000000-0000-4000-8000-000000000009',
    revision: 3,
    agreement_id: null,
  };
  command.change.values.intended_recipient = { kind: 'known', value: person };
  command.change.values.provenance = {
    kind: 'hearing_result',
    reference: hearing,
    locator: 'Sesion anterior',
    support,
  };
  const prepared = factPrepared(command);
  prepared.sources.participants = [
    {
      case_id: factCaseId,
      id: person.id,
      revision: 2,
      values_digest: factDigest,
      directory_status: 'archived',
      subject: null,
      display_name: 'Persona historica',
      procedural_role: 'Testigo',
      organization: null,
      kind: null,
    },
  ];
  prepared.sources.hearing_results = [
    {
      ...hearing,
      case_id: factCaseId,
      values_digest: factDigest,
      submission_digest: 'b'.repeat(64),
      status: 'withdrawn',
      occurrence: 'occurred',
      event_time: { precision: 'date', year: 2026, month: 9, day: 1, offset_seconds: -21600 },
      summary: 'Sesion historica',
      agreement: null,
    },
  ];
  prepared.sources.direct_supports = [
    {
      document_id: support.document_id,
      version: 1,
      digest: factDigest,
      name: 'constancia.pdf',
      format: 'pdf',
      policy: 'pdf_docx_v1',
    },
  ];
  return prepared;
}
