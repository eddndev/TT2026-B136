import {
  factPrepared,
  factCommandFixture,
  factCaseId,
  factActorId,
} from './procedural-fact-unit.mjs';
export const resourceCaseId = factCaseId;
export const resourceActor = { id: factActorId, email: 'captured@example.test' };
export const resourceId = 'a0000000-0000-4000-8000-000000000001';
export const resourceActId = 'b0000000-0000-4000-8000-000000000001';
export const resourceOperationId = 'c0000000-0000-4000-8000-000000000001';
export const unknown = () => ({ kind: 'unknown', reason: 'No consta' });
export function resourceValuesFixture() {
  return {
    kind: 'appeal',
    mode: { kind: 'known', value: 'written' },
    title: 'Recurso declarado',
    resolution: { id: '30000000-0000-4000-8000-000000000003', revision: 1 },
    resolution_evidence: {
      document_id: 'd0000000-0000-4000-8000-000000000001',
      version: 2,
      digest: 'a'.repeat(64),
      locator: 'Pagina 1',
    },
    resolution_reference: unknown(),
    issuing_authority: unknown(),
    receiving_authority: null,
    resolution_at: { precision: 'unknown' },
    notification_at: null,
    challenged_part: 'Extremo impugnado',
    grounds: 'Motivos declarados',
    appellants: [{ name: 'Persona declarada', role: unknown(), participant: null }],
  };
}
export function resourceActFixture() {
  return {
    kind: 'withdrawal',
    mode: unknown(),
    occurred_at: { precision: 'unknown' },
    authority: unknown(),
    statement: 'Desistimiento declarado',
    evidence: [resourceValuesFixture().resolution_evidence],
  };
}
export function resourceCommandFixture(action = 'register') {
  return {
    operation_id: resourceOperationId,
    resource_id: resourceId,
    change: {
      action,
      expected_revision: action === 'register' ? 0 : 3,
      ...(['register', 'correct'].includes(action) ? { values: resourceValuesFixture() } : {}),
      ...(['record_act', 'correct_act'].includes(action)
        ? { act_id: resourceActId, values: resourceActFixture() }
        : {}),
      ...(['correct', 'correct_act', 'archive', 'reactivate'].includes(action)
        ? { reason: 'Motivo declarado' }
        : {}),
      ...(action === 'correct_act' ? { expected_act_revision: 1 } : {}),
    },
  };
}
const prior = (revision) => ({ revision, capture_digest: 'b'.repeat(64) });
export function resourcePrepared(command = resourceCommandFixture()) {
  const fact = factPrepared(factCommandFixture('notification'));
  const values = ['register', 'correct'].includes(command.change.action)
    ? command.change.values
    : resourceValuesFixture();
  const support = values.resolution_evidence;
  const supports = [
    {
      document_id: support.document_id,
      version: support.version,
      digest: support.digest,
      name: 'constancia.pdf',
      format: 'pdf',
      policy: 'pdf_docx_v1',
    },
  ];
  const isAct = ['record_act', 'correct_act'].includes(command.change.action);
  return {
    case_id: resourceCaseId,
    command: structuredClone(command),
    result_revision: command.change.expected_revision + 1,
    values: structuredClone(values),
    status: command.change.action === 'archive' ? 'archived' : 'active',
    sources: { resolution: fact.sources.resolution, appellants: [], supports },
    act: isAct
      ? {
          id: command.change.act_id,
          revision: command.change.expected_act_revision ? 2 : 1,
          values: structuredClone(command.change.values),
          supports: structuredClone(supports),
          previous: command.change.action === 'correct_act' ? prior(2) : null,
        }
      : null,
    previous: command.change.expected_revision ? prior(command.change.expected_revision) : null,
    recorded_by: { ...resourceActor },
    observed_administration: fact.observed_administration,
    observed_stage: { case_id: resourceCaseId, current: null },
    submission_digest: 'c'.repeat(64),
  };
}
export function resourceRecord(prepared = resourcePrepared()) {
  const { command } = prepared;
  return {
    case_id: prepared.case_id,
    id: command.resource_id,
    revision: prepared.result_revision,
    values: structuredClone(prepared.values),
    status: prepared.status,
    sources: structuredClone(prepared.sources),
    act: structuredClone(prepared.act),
    reason: command.change.reason ?? null,
    recorded_by: { ...prepared.recorded_by },
    recorded_at: '2026-09-19T10:00:00Z',
    recorded_administration: structuredClone(prepared.observed_administration),
    recorded_stage: structuredClone(prepared.observed_stage),
    receipt: {
      action: command.change.action,
      operation_id: command.operation_id,
      expected_revision: command.change.expected_revision,
      previous: structuredClone(prepared.previous),
      values_digest: 'd'.repeat(64),
      sources_digest: 'e'.repeat(64),
      submission_digest: prepared.submission_digest,
      capture_digest: 'f'.repeat(64),
    },
  };
}
