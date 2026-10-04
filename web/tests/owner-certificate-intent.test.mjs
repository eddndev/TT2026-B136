import test from 'node:test';
import assert from 'node:assert/strict';
import {
  registrationIntent,
  withdrawalIntent,
  reconcileOwnerIntent,
  uncertainOwnerWrite,
} from '../src/lib/owner-certificate-intent.mjs';
import {
  bindingId,
  ownerId,
  signature,
  preparation,
  receipt,
  submission,
  withdrawn,
} from './fixtures/owner-certificates.mjs';

test('registration captures one immutable public command rather than references to editable preparation', () => {
  const prepared = preparation(),
    expected = submission(prepared);
  const command = registrationIntent(prepared, signature.toString('base64'));
  assert.deepEqual(command, { ownerId, bindingId, operation: 'register', data: expected });
  assert.ok(Object.isFrozen(command) && Object.isFrozen(command.data));
  prepared.statement_base64 = 'changed';
  prepared.certificate.der_base64 = 'changed';
  assert.deepEqual(command.data, expected);
  const persisted = JSON.parse(JSON.stringify(command));
  assert.deepEqual(persisted, command);
  for (const size of [383, 385])
    assert.throws(() => registrationIntent(preparation(), Buffer.alloc(size).toString('base64')));
});

test('reconciliation compares exact statement, DER and signature and preserves terminal registration evidence', () => {
  const original = receipt(),
    command = registrationIntent(preparation(), signature.toString('base64'));
  assert.equal(reconcileOwnerIntent(command, null), 'absent');
  assert.equal(reconcileOwnerIntent(command, original), 'matched');
  assert.equal(reconcileOwnerIntent(command, withdrawn(original)), 'matched');
  for (const mutate of [
    (value) => {
      value.owner_id = bindingId;
    },
    (value) => {
      value.binding_id = ownerId;
    },
    (value) => {
      value.registration.statement_base64 = Buffer.alloc(150, 8).toString('base64');
    },
    (value) => {
      value.registration.certificate.der_base64 =
        Buffer.from('different-public-DER').toString('base64');
    },
    (value) => {
      value.registration.signature_base64 = Buffer.alloc(384, 8).toString('base64');
    },
  ]) {
    const changed = structuredClone(original);
    mutate(changed);
    assert.equal(reconcileOwnerIntent(command, changed), 'conflict');
  }
  assert.deepEqual(command.data, submission());
});

test('withdrawal freezes revision one and reconciles only the same original registration with a terminal receipt', () => {
  const original = receipt(),
    command = withdrawalIntent(original);
  assert.deepEqual(command, {
    ownerId,
    bindingId,
    operation: 'withdraw',
    data: { expected_revision: 1 },
    registration: submission(),
  });
  assert.equal(reconcileOwnerIntent(command, original), 'unconfirmed');
  assert.equal(reconcileOwnerIntent(command, withdrawn(original)), 'matched');
  const other = withdrawn(original);
  other.registration.signature_base64 = Buffer.alloc(384, 9).toString('base64');
  assert.equal(reconcileOwnerIntent(command, other), 'conflict');
  assert.throws(() => withdrawalIntent(withdrawn(original)));
  assert.ok(Object.isFrozen(command.data) && Object.isFrozen(command.registration));
});

test('loss of authority or transport after submission remains uncertain and never creates a replacement command', () => {
  const command = registrationIntent(preparation(), signature.toString('base64'));
  for (const error of [
    { status: 401 },
    { status: 403 },
    { status: 500 },
    { status: 503 },
    new Error('network'),
  ])
    assert.equal(uncertainOwnerWrite(error), true);
  for (const status of [400, 409, 413, 422]) assert.equal(uncertainOwnerWrite({ status }), false);
  assert.deepEqual(command, { ownerId, bindingId, operation: 'register', data: submission() });
});
