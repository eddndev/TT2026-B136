import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { credentialSelection } from '../src/lib/participant-credential-selection.mjs';

const pem = readFileSync(new URL('./fixtures/participant-public-certificate.pem', import.meta.url));

async function capturedCertificate(name = 'public.pem') {
  const selection = credentialSelection();
  selection.updateContext('original owner', 'original values');
  assert.equal(await selection.selectCertificate(new File([pem], name)), true);
  const draft = selection.captureDraft();
  selection.dispose();
  return draft;
}

function heldRead(blob) {
  const read = blob.arrayBuffer.bind(blob);
  let release, entered;
  const pending = new Promise((resolve) => {
    release = resolve;
  });
  const started = new Promise((resolve) => {
    entered = resolve;
  });
  Object.defineProperty(blob, 'arrayBuffer', {
    value() {
      entered();
      return pending;
    },
  });
  return { started, release: async () => release(await read()) };
}

function assertEmpty(selection) {
  const state = selection.snapshot();
  assert.deepEqual(
    [state.certificate, state.prepared, state.signature, state.retained],
    [null, null, null, null],
  );
  assert.equal(state.ready, false);
  assert.equal(state.busy, false);
  assert.equal(selection.captureDraft(), null);
}

test('revoked draft admission rejects a public certificate whose read was already pending', async (t) => {
  const draft = await capturedCertificate();
  const read = heldRead(draft.certificate.blob);
  const selection = credentialSelection();
  t.after(() => selection.dispose());
  selection.updateContext('same owner', 'fresh values');
  let admitted = true;
  const restoring = selection.restoreDraft(draft, () => admitted);
  await read.started;
  assert.equal(selection.snapshot().busy, true);
  assert.equal(selection.snapshot().certificate, null);
  admitted = false;
  await read.release();
  assert.equal(await restoring, false);
  assertEmpty(selection);
});

test('a second restoration remains authoritative when an older certificate read completes', async (t) => {
  const first = await capturedCertificate('first.pem');
  const second = await capturedCertificate('replacement.pem');
  const read = heldRead(first.certificate.blob);
  const selection = credentialSelection();
  t.after(() => selection.dispose());
  selection.updateContext('same owner', 'fresh values');
  const restoring = selection.restoreDraft(first, () => true);
  await read.started;
  assert.equal(await selection.restoreDraft(second, () => true), true);
  const replacement = selection.snapshot().certificate;
  assert.equal(replacement.name, 'replacement.pem');
  await read.release();
  assert.equal(await restoring, false);
  assert.equal(selection.snapshot().certificate, replacement);
  assert.deepEqual(Buffer.from(await replacement.blob.arrayBuffer()), pem);
  assert.deepEqual(
    [selection.snapshot().prepared, selection.snapshot().signature, selection.snapshot().retained],
    [null, null, null],
  );
  assert.equal(selection.snapshot().ready, false);
  assert.equal(selection.snapshot().busy, false);
});

test('an explicit empty restoration supersedes a pending certificate read', async (t) => {
  const draft = await capturedCertificate();
  const read = heldRead(draft.certificate.blob);
  const selection = credentialSelection();
  t.after(() => selection.dispose());
  selection.updateContext('same owner', 'fresh values');
  const restoring = selection.restoreDraft(draft, () => true);
  await read.started;
  assert.equal(await selection.restoreDraft(null, () => true), true);
  await read.release();
  assert.equal(await restoring, false);
  assertEmpty(selection);
});

test('clearing the owner prevents a delayed restoration from reviving public materials', async (t) => {
  const draft = await capturedCertificate();
  const read = heldRead(draft.certificate.blob);
  const selection = credentialSelection();
  t.after(() => selection.dispose());
  selection.updateContext('previous owner', 'fresh values');
  const restoring = selection.restoreDraft(draft, () => true);
  await read.started;
  selection.updateContext(null, null);
  await read.release();
  assert.equal(await restoring, false);
  assert.equal(selection.snapshot().available, false);
  assertEmpty(selection);
  selection.updateContext('another owner', 'other values');
  assertEmpty(selection);
});
