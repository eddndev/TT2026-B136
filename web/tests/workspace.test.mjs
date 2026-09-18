import test from 'node:test';
import assert from 'node:assert/strict';
import { documentStatus, normalizeView } from '../src/lib/workspace.mjs';

test('document metadata distinguishes sealed, pending and unknown status', () => {
  assert.equal(documentStatus({ sealed: true }).key, 'sealed');
  assert.equal(documentStatus({ sealed: false }).key, 'pending');
  assert.equal(documentStatus({}).label, 'Estado por confirmar');
});

test('failed verification takes priority over a known seal', () => {
  assert.deepEqual(documentStatus({ sealed: true, report: { verdict: 'not_valid' } }), {
    label: 'Revisar evidencia',
    tone: 'danger',
    key: 'failed',
  });
  assert.equal(documentStatus({ sealed: true, report: { verdict: 'valid' } }).key, 'verified');
});

test('case navigation is available to every recognized role including clients', () => {
  for (const role of ['owner', 'litigator', 'paralegal', 'client']) {
    assert.equal(normalizeView('#cases', role), 'cases');
  }
});

test('unknown locations and owner-only locations resolve safely for each role', () => {
  assert.equal(normalizeView('#team', 'owner'), 'team');
  assert.equal(normalizeView('#team', 'paralegal'), 'overview');
  assert.equal(normalizeView('#unknown', 'owner'), 'overview');
  assert.equal(normalizeView('#documents', 'client'), 'documents');
});

test('judicial calendars are a global staff location with no Client fallback into case context', () => {
  for (const role of ['owner', 'litigator', 'paralegal'])
    assert.equal(normalizeView('#judicial-calendars', role), 'judicial-calendars');
  assert.equal(normalizeView('#judicial-calendars', 'client'), 'overview');
});

test('declared facts use an authorized case location for staff only', () => {
  for (const role of ['owner', 'litigator', 'paralegal'])
    assert.equal(normalizeView('#resolutions', role), 'resolutions');
  for (const role of ['client', '', undefined])
    assert.equal(normalizeView('#resolutions', role), 'overview');
});

test('deadlines are an assigned case view for staff and never a Client fallback', () => {
  for (const role of ['owner', 'litigator', 'paralegal'])
    assert.equal(normalizeView('#deadlines', role), 'deadlines');
  for (const role of ['client', '', undefined])
    assert.equal(normalizeView('#deadlines', role), 'overview');
});
