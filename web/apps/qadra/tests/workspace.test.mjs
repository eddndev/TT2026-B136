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

test('case navigation is available to every staff role and hidden from clients', () => {
  for (const role of ['owner', 'litigator', 'paralegal']) {
    assert.equal(normalizeView('#cases', role), 'cases');
  }
  assert.equal(normalizeView('#cases', 'client'), 'contracts');
});

test('a client reaches only its contracts, its signature and the evidence validator', () => {
  for (const view of ['contracts', 'signing-certificates', 'evidence-validation'])
    assert.equal(normalizeView(`#${view}`, 'client'), view);
  for (const view of ['overview', 'cases', 'documents', 'case-summary', 'guide', 'agenda', 'team'])
    assert.equal(normalizeView(`#${view}`, 'client'), 'contracts');
});

test('the signing binding is open to Owner and Litigator, and contracts to signers', () => {
  for (const role of ['owner', 'litigator']) {
    assert.equal(normalizeView('#signing-certificates', role), 'signing-certificates');
    assert.equal(normalizeView('#contracts', role), 'contracts');
    assert.equal(normalizeView('#case-contracts', role), 'case-contracts');
  }
  assert.equal(normalizeView('#signing-certificates', 'paralegal'), 'overview');
  assert.equal(normalizeView('#contracts', 'paralegal'), 'overview');
  assert.equal(normalizeView('#case-contracts', 'paralegal'), 'case-contracts');
  assert.equal(normalizeView('#case-contracts', 'client'), 'contracts');
});

test('unknown locations and owner-only locations resolve safely for each role', () => {
  assert.equal(normalizeView('#team', 'owner'), 'team');
  assert.equal(normalizeView('#team', 'paralegal'), 'overview');
  assert.equal(normalizeView('#unknown', 'owner'), 'overview');
  assert.equal(normalizeView('#documents', 'client'), 'contracts');
});

test('judicial calendars are a global staff location with no Client fallback into case context', () => {
  for (const role of ['owner', 'litigator', 'paralegal'])
    assert.equal(normalizeView('#judicial-calendars', role), 'judicial-calendars');
  assert.equal(normalizeView('#judicial-calendars', 'client'), 'contracts');
});

test('declared facts use an authorized case location for staff only', () => {
  for (const role of ['owner', 'litigator', 'paralegal'])
    assert.equal(normalizeView('#resolutions', role), 'resolutions');
  for (const role of ['', undefined]) assert.equal(normalizeView('#resolutions', role), 'overview');
  assert.equal(normalizeView('#resolutions', 'client'), 'contracts');
});

test('deadlines are an assigned case view for staff and never a Client fallback', () => {
  for (const role of ['owner', 'litigator', 'paralegal'])
    assert.equal(normalizeView('#deadlines', role), 'deadlines');
  for (const role of ['', undefined]) assert.equal(normalizeView('#deadlines', role), 'overview');
  assert.equal(normalizeView('#deadlines', 'client'), 'contracts');
});
