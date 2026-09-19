import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  resourceValues,
  resourceActValues,
  resourceCommand,
} from '../src/lib/procedural-resource-values.mjs';
const id = '00000000-0000-4000-8000-000000000001';
const unknown = () => ({ kind: 'unknown', reason: 'No consta en la fuente' });
const support = () => ({
  document_id: id,
  version: 2,
  digest: 'a'.repeat(64),
  locator: 'Pagina 1',
});
const values = () => ({
  kind: 'revocation',
  mode: unknown(),
  title: 'Revocacion declarada',
  resolution: { id, revision: 3 },
  resolution_evidence: support(),
  resolution_reference: unknown(),
  issuing_authority: unknown(),
  receiving_authority: null,
  resolution_at: { precision: 'unknown' },
  notification_at: null,
  challenged_part: 'Parte impugnada',
  grounds: 'Motivos declarados',
  appellants: [{ name: 'Persona declarada', role: unknown(), participant: null }],
});
test('resource values preserve unknown and absent declarations without inventing times', () => {
  const raw = values();
  assert.deepEqual(resourceValues(raw), raw);
  raw.kind = 'appeal';
  raw.mode = { kind: 'known', value: 'written' };
  assert.deepEqual(resourceValues(raw), raw);
  for (const key of ['resolution', 'resolution_evidence', 'appellants']) {
    const bad = values();
    delete bad[key];
    assert.throws(() => resourceValues(bad));
  }
});
test('resource source selections stay exact and names are not identities', () => {
  const raw = values();
  raw.appellants.push({ ...raw.appellants[0] });
  assert.equal(resourceValues(raw).appellants.length, 2);
  raw.appellants.forEach((row) => {
    row.participant = { id, revision: 2 };
  });
  assert.throws(() => resourceValues(raw));
  assert.throws(() => resourceValues({ ...values(), resolution: { id, revision: 0 } }));
  assert.throws(() => resourceValues({ ...values(), due_at: '2030-01-01' }));
});
test('act evidence retains independent locators but rejects conflicting content digests', () => {
  const raw = {
    kind: 'interposition',
    mode: { kind: 'known', value: 'oral' },
    occurred_at: {
      precision: 'minute',
      year: 2026,
      month: 9,
      day: 20,
      hour: 9,
      minute: 15,
      offset_seconds: -21600,
    },
    authority: unknown(),
    statement: 'Constancia del acto',
    evidence: [support(), { ...support(), locator: 'Pagina 2' }],
  };
  assert.deepEqual(resourceActValues(raw), raw);
  assert.throws(() => resourceActValues({ ...raw, evidence: [] }));
  assert.throws(() => resourceActValues({ ...raw, evidence: [...raw.evidence, support()] }));
  raw.evidence[1].digest = 'b'.repeat(64);
  assert.throws(() => resourceActValues(raw));
});
test('commands bind resource and act revisions and separate archive from withdrawal', () => {
  const raw = {
    operation_id: id,
    resource_id: id,
    change: { action: 'register', expected_revision: 0, values: values() },
  };
  assert.deepEqual(resourceCommand(raw), raw);
  assert.throws(() => resourceCommand({ ...raw, change: { ...raw.change, expected_revision: 1 } }));
  const archive = {
    ...raw,
    change: { action: 'archive', expected_revision: 2, reason: 'Archivo administrativo' },
  };
  assert.deepEqual(resourceCommand(archive), archive);
  assert.throws(() =>
    resourceCommand({ ...archive, change: { ...archive.change, action: 'withdrawal' } }),
  );
  assert.throws(() =>
    resourceCommand({ ...archive, change: { ...archive.change, expected_revision: 4294967295 } }),
  );
});
