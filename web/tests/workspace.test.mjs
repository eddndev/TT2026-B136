import test from 'node:test';
import assert from 'node:assert/strict';
import {
  documentStatus,
  sessionStats,
  filterDocuments,
  normalizeView,
} from '../src/lib/workspace.mjs';

test('overview only counts known session records and distinguishes unconfirmed seals', () => {
  const records = [{ id: 'a', sealed: false }, { id: 'b', sealed: true }, { id: 'c' }];
  assert.deepEqual(sessionStats(records), { total: 3, sealed: 1, pending: 1, verified: 0 });
  assert.equal(documentStatus(records[2]).label, 'Estado por confirmar');
});

test('failed verification takes priority over a known seal', () => {
  assert.deepEqual(documentStatus({ sealed: true, report: { verdict: 'not_valid' } }), {
    label: 'Revisar evidencia',
    tone: 'danger',
    key: 'failed',
  });
  assert.equal(documentStatus({ sealed: true, report: { verdict: 'valid' } }).key, 'verified');
});

test('search and status filters combine without mutating the session list', () => {
  const records = [
    { id: 'a', name: 'Contrato.pdf', sealed: false },
    { id: 'b', name: 'Demanda.pdf', sealed: true },
  ];
  assert.deepEqual(filterDocuments(records, 'contrato', 'pending'), [records[0]]);
  assert.deepEqual(filterDocuments(records, 'contrato', 'sealed'), []);
  assert.equal(records.length, 2);
});

test('unknown locations and owner-only locations resolve safely for each role', () => {
  assert.equal(normalizeView('#team', 'owner'), 'team');
  assert.equal(normalizeView('#team', 'paralegal'), 'overview');
  assert.equal(normalizeView('#unknown', 'owner'), 'overview');
  assert.equal(normalizeView('#documents', 'client'), 'documents');
});
