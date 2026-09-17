import { readFileSync } from 'node:fs';
import { hearingCaseId } from './hearings.mjs';
export const factCaseId = hearingCaseId;
export const resolutionId = '10000000-0000-4000-8000-000000000011';
export const notificationId = '10000000-0000-4000-8000-000000000012';
const corpus = JSON.parse(
  readFileSync(
    new URL('../../../crates/domain/tests/fixtures/procedural_fact_vectors.json', import.meta.url),
    'utf8',
  ),
);
export const factVector = (name) =>
  structuredClone(corpus.find((row) => row.name === name).normalized);
export function factValues(family = 'resolution') {
  const values = factVector(`${family}_minimum`);
  values.summary =
    family === 'resolution' ? 'Resolucion declarada historica' : 'Notificacion declarada historica';
  values.provenance.note = 'Captura informada por la operadora';
  if (family === 'notification') values.resolution = { id: resolutionId, revision: 1 };
  return values;
}
export function factCommand(family = 'resolution', action = 'record', expected_revision = 0) {
  return {
    family,
    operation_id: '10000000-0000-4000-8000-000000000013',
    id: family === 'resolution' ? resolutionId : notificationId,
    ...(family === 'notification' ? { resolution_id: resolutionId } : {}),
    change: {
      action,
      expected_revision,
      ...(action === 'record' ? {} : { reason: 'Precision declarada' }),
      ...(action === 'withdraw' ? {} : { values: factValues(family) }),
    },
  };
}
export const factAdministration = {
  kind: 'recorded',
  case_id: factCaseId,
  revision: 1,
  title: 'Defensa inicial',
  reference: 'NUC-123',
  status: 'active',
  values_digest: 'c'.repeat(64),
  changed_at: '2026-09-16T12:00:00Z',
  changed_by: { id: 'user', email: 'hatz@example.com' },
};
export const emptyFactSources = () => ({
  resolution: null,
  participants: [],
  hearing_results: [],
  direct_supports: [],
});
export function factResolutionSource(row) {
  return {
    case_id: row.case_id,
    id: row.id,
    revision: row.revision,
    values_digest: row.values_digest,
    submission_digest: row.receipt.submission_digest,
    status: row.status,
    class: structuredClone(row.values.class),
    issuer: structuredClone(row.values.issuer),
    issued_at: structuredClone(row.values.issued_at),
    summary: row.values.summary,
  };
}
export function factPrepared(command = factCommand(), base = null, sources = emptyFactSources()) {
  return {
    case_id: factCaseId,
    actor_id: 'user',
    command: structuredClone(command),
    result_revision: command.change.expected_revision + 1,
    values: structuredClone(command.change.values || base.values),
    values_digest: 'd'.repeat(64),
    sources: structuredClone(sources),
    sources_digest: 'e'.repeat(64),
    submission_digest: 'f'.repeat(64),
    observed_administration: structuredClone(factAdministration),
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
    values: structuredClone(prepared.values),
    values_digest: prepared.values_digest,
    status: command.change.action === 'withdraw' ? 'withdrawn' : 'recorded',
    reason: command.change.reason || null,
    receipt: {
      operation_id: command.operation_id,
      action: command.change.action,
      expected_revision: command.change.expected_revision,
      sources_digest: prepared.sources_digest,
      submission_digest: prepared.submission_digest,
    },
    recorded_administration: structuredClone(prepared.observed_administration),
    recorded_at: '2026-09-16T12:00:00Z',
    recorded_by: { id: prepared.actor_id, email: 'hatz@example.com' },
    sources: structuredClone(prepared.sources),
  };
}
export function factRow(row) {
  return {
    case_id: row.case_id,
    family: row.family,
    id: row.id,
    revision: row.revision,
    status: row.status,
    ...(row.family === 'resolution'
      ? { class: row.values.class, issued_at: row.values.issued_at }
      : {
          resolution_id: row.resolution_id,
          resolution: row.values.resolution,
          outcome: row.values.outcome,
          practiced_at: row.values.practiced_at,
        }),
  };
}
export function factHistoryRow(row) {
  const { values, sources, ...history } = row;
  return history;
}
export const factKey = (row) => `${row.family}:${row.resolution_id || ''}:${row.id}`;
