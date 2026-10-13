import test from 'node:test';
import assert from 'node:assert/strict';
import { createCaOperations } from '../src/lib/ca-operations-model.mjs';

const operationId = '10000000-0000-4000-8000-000000000001';
const otherId = '10000000-0000-4000-8000-000000000002';
const fingerprint = 'a'.repeat(64);
const rootFingerprint = 'b'.repeat(64);
const csr = '-----BEGIN CERTIFICATE REQUEST-----\nPUBLIC\n-----END CERTIFICATE REQUEST-----\n';
const certificate = '-----BEGIN CERTIFICATE-----\nPUBLIC\n-----END CERTIFICATE-----\n';
const draft = { csr_pem: csr, purpose: 'partner', holder: 'Reviewed Holder',
  reference: 'Public reference', criterion: 'Possession and holder reviewed' };
const preview = { csr_sha256: 'c'.repeat(64), spki_sha256: 'd'.repeat(64),
  root_fingerprint: rootFingerprint, subject: 'CN=Reviewed Holder', purpose: 'partner' };
const receipt = { operation_id: operationId, serial: '1000', fingerprint, purpose: 'partner',
  certificate_pem: certificate, recorded_at: 100, operator_id: otherId,
  holder: draft.holder, reference: draft.reference, criterion: draft.criterion };
const inventoryItem = { serial: '1000', fingerprint, subject: preview.subject,
  purpose: 'partner', root_fingerprint: rootFingerprint, not_before: 90, not_after: 200,
  validity: 'valid', local_status: 'issued', published_status: 'unknown' };
const historyItem = { operation_id: operationId, kind: 'issue', state: 'pending',
  recorded_at: 100, operator_id: otherId, holder: draft.holder,
  reference: draft.reference, criterion: draft.criterion, serial: null, fingerprint: null };

function fixture(respond = (payload) => payload.action === 'preview' ? preview : receipt) {
  const calls = [];
  let uuidCalls = 0;
  const model = createCaOperations({
    oncommand: async (payload) => { calls.push(structuredClone(payload)); return respond(payload); },
    uuid: () => { uuidCalls++; return operationId; },
  });
  return { model, calls, uuidCalls: () => uuidCalls };
}

function deferred() {
  let resolve;
  const promise = new Promise((done) => { resolve = done; });
  return { promise, resolve };
}

test('public request requires preview and a separate confirmation before issuing', async () => {
  const { model, calls, uuidCalls } = fixture();
  model.edit(draft);
  await model.confirm();
  assert.equal(calls.length, 0);
  await model.preview();
  assert.deepEqual(calls, [{ action: 'preview', csr_pem: csr, purpose: 'partner' }]);
  assert.equal(model.snapshot().phase, 'review');
  assert.equal(uuidCalls(), 0);
  await model.confirm();
  assert.deepEqual(calls[1], { action: 'issue', operation_id: operationId, ...draft,
    expected_root_fingerprint: rootFingerprint });
  assert.equal(uuidCalls(), 1);
  assert.equal(model.snapshot().phase, 'completed');
  assert.deepEqual(model.snapshot().receipt, receipt);
});

test('documentary purpose stays distinct through preview and issue', async () => {
  const { model, calls } = fixture((payload) => payload.action === 'preview'
    ? { ...preview, purpose: 'document-signing' } : { ...receipt, purpose: 'document-signing' });
  model.edit({ ...draft, purpose: 'document-signing' });
  await model.preview();
  await model.confirm();
  assert.equal(calls[0].purpose, 'document-signing');
  assert.equal(calls[1].purpose, 'document-signing');
});

test('editing any reviewed field invalidates the confirmation', async () => {
  const { model, calls } = fixture();
  model.edit(draft);
  await model.preview();
  model.edit({ criterion: 'Another review' });
  assert.equal(model.snapshot().preview, null);
  assert.equal(model.snapshot().phase, 'draft');
  await model.confirm();
  assert.equal(calls.length, 1);
});

test('late preview cannot replace an edited public request', async () => {
  const pending = deferred();
  const { model } = fixture(() => pending.promise);
  model.edit(draft);
  const request = model.preview();
  model.edit({ holder: 'Different Holder' });
  pending.resolve(preview);
  await request;
  assert.equal(model.snapshot().phase, 'draft');
  assert.equal(model.snapshot().preview, null);
});

test('invalid public input or empty review text does not call the server', async () => {
  const { model, calls } = fixture();
  for (const override of [{ csr_pem: '' }, { csr_pem: 'x'.repeat(16385) },
    { csr_pem: '-----BEGIN PRIVATE KEY-----\nSECRET' }, { purpose: 'root' },
    { holder: '' }, { reference: 'x'.repeat(501) }, { criterion: 'a\nb' }]) {
    model.edit({ ...draft, ...override });
    await model.preview();
    assert.equal(model.snapshot().preview, null);
    assert.ok(model.snapshot().error);
  }
  assert.equal(calls.length, 0);
});

test('preview purpose mismatch never enables issuance', async () => {
  const { model, calls } = fixture(() => ({ ...preview, purpose: 'document-signing' }));
  model.edit(draft);
  await model.preview();
  await model.confirm();
  assert.equal(model.snapshot().phase, 'draft');
  assert.equal(calls.length, 1);
});

test('uncertain response preserves one operation and resumes without reissuing', async () => {
  const { model, calls, uuidCalls } = fixture((payload) => {
    if (payload.action === 'preview') return preview;
    if (payload.action === 'issue') throw new Error('private diagnostic');
    return receipt;
  });
  model.edit(draft);
  await model.preview();
  await model.confirm();
  assert.equal(model.snapshot().phase, 'uncertain');
  assert.equal(model.snapshot().operation_id, operationId);
  assert.doesNotMatch(model.snapshot().error, /private diagnostic/);
  model.edit({ holder: 'Unsafe replacement' });
  model.startNew();
  await model.confirm();
  assert.equal(model.snapshot().draft.holder, draft.holder);
  assert.equal(calls.length, 2);
  await model.resume(operationId);
  assert.deepEqual(calls[2], { action: 'resume-issue', operation_id: operationId });
  assert.equal(model.snapshot().phase, 'completed');
  assert.equal(uuidCalls(), 1);
});

test('double confirmation sends one signing command', async () => {
  const pending = deferred();
  const { model, calls } = fixture((payload) => payload.action === 'preview' ? preview : pending.promise);
  model.edit(draft);
  await model.preview();
  const first = model.confirm();
  await model.confirm();
  assert.equal(calls.filter((call) => call.action === 'issue').length, 1);
  pending.resolve(receipt);
  await first;
});

test('wrong operation receipt leaves recovery required', async () => {
  const { model } = fixture((payload) => payload.action === 'preview'
    ? preview : { ...receipt, operation_id: otherId });
  model.edit(draft);
  await model.preview();
  await model.confirm();
  assert.equal(model.snapshot().phase, 'uncertain');
  assert.equal(model.snapshot().receipt, null);
});

test('a completed receipt allows an explicit new public request', async () => {
  const { model } = fixture();
  model.edit(draft);
  await model.preview();
  await model.confirm();
  model.startNew();
  assert.equal(model.snapshot().phase, 'draft');
  assert.equal(model.snapshot().operation_id, null);
  assert.equal(model.snapshot().draft.csr_pem, '');
});

test('inventory pagination passes bounded offsets and leaves publication unknown', async () => {
  const { model, calls } = fixture(() => ({ items: [inventoryItem], total: 21, offset: 20, limit: 20 }));
  await model.loadInventory(20);
  assert.deepEqual(calls, [{ action: 'inventory', offset: 20, limit: 20 }]);
  assert.deepEqual(model.snapshot().inventory.items, [inventoryItem]);
  assert.equal(model.snapshot().inventory.total, 21);
  assert.equal(model.snapshot().inventory.items[0].published_status, 'unknown');
  await model.loadInventory(-1);
  assert.equal(calls.length, 1);
});

test('download requests exact serial plus fingerprint and preserves certificate text', async () => {
  const pem = '\n' + certificate;
  const { model, calls } = fixture(() => ({ certificate_pem: pem, serial: '1000', fingerprint }));
  const result = await model.download(inventoryItem);
  assert.deepEqual(calls, [{ action: 'certificate', serial: '1000', fingerprint }]);
  assert.deepEqual(result, { certificate_pem: pem, serial: '1000', fingerprint });
});

test('download refuses a response for another certificate', async () => {
  const { model } = fixture(() => ({ certificate_pem: certificate, serial: '1001', fingerprint }));
  assert.equal(await model.download(inventoryItem), null);
  assert.ok(model.snapshot().error);
});

test('fresh authenticated state can resume a pending issuance from history', async () => {
  const { model, calls, uuidCalls } = fixture((payload) => payload.action === 'history'
    ? { items: [historyItem], total: 1, offset: 0, limit: 20 } : receipt);
  await model.loadHistory();
  assert.equal(model.snapshot().history.items[0].state, 'pending');
  await model.resume(historyItem.operation_id);
  assert.deepEqual(calls[1], { action: 'resume-issue', operation_id: operationId });
  assert.equal(model.snapshot().phase, 'completed');
  assert.equal(uuidCalls(), 0);
});

test('history cannot replace an unresolved local operation with another issuance', async () => {
  const { model, calls } = fixture((payload) => {
    if (payload.action === 'preview') return preview;
    throw new Error('unavailable');
  });
  model.edit(draft);
  await model.preview();
  await model.confirm();
  await model.resume(otherId);
  assert.equal(calls.length, 2);
  assert.equal(model.snapshot().operation_id, operationId);
});

test('disposal on session loss clears material and ignores a delayed signing response', async () => {
  const pending = deferred();
  const { model } = fixture((payload) => payload.action === 'preview' ? preview : pending.promise);
  model.edit(draft);
  await model.preview();
  const signing = model.confirm();
  model.dispose();
  pending.resolve(receipt);
  await signing;
  assert.equal(model.snapshot().receipt, null);
  assert.equal(model.snapshot().draft.csr_pem, '');
  assert.equal(model.snapshot().operation_id, null);
});

test('snapshot mutations cannot alter the confirmed request or expose transport state', async () => {
  const { model, calls } = fixture();
  model.edit(draft);
  await model.preview();
  const state = model.snapshot();
  state.draft.holder = 'Injected';
  state.preview.root_fingerprint = 'e'.repeat(64);
  await model.confirm();
  assert.equal(calls[1].holder, draft.holder);
  assert.equal(calls[1].expected_root_fingerprint, rootFingerprint);
  assert.doesNotMatch(JSON.stringify(model.snapshot()), /token|password|challenge|private_key/);
});
