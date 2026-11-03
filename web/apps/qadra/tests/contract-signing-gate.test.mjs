import test from 'node:test';
import assert from 'node:assert/strict';
import { messages } from '../src/lib/api-messages.mjs';
import { signRefusal } from '../src/lib/contract-signing-gate.mjs';

const failure = (status, code) => Object.assign(new Error(code), { status, code });

test('a stale statement is discarded so the party prepares a fresh one', () => {
  for (const code of ['signing_account_changed', 'signing_trust_changed']) {
    assert.deepEqual(signRefusal(failure(409, code)), {
      mode: 'editing',
      keepSignature: false,
      stale: true,
      reload: false,
    });
  }
  assert.deepEqual(signRefusal(failure(409, 'contract_revision_conflict')), {
    mode: 'editing',
    keepSignature: false,
    stale: true,
    reload: true,
  });
});

test('a refusal that wrote nothing keeps the signature for a later resend', () => {
  for (const code of ['contract_timestamp_unavailable', 'signing_trust_unavailable']) {
    assert.deepEqual(signRefusal(failure(503, code)), {
      mode: 'editing',
      keepSignature: true,
      stale: false,
      reload: false,
    });
    assert.equal(typeof messages[code], 'string', code);
  }
});

test('other answers keep their previous handling', () => {
  assert.deepEqual(signRefusal(failure(409, 'signing_conflict')), {
    mode: 'editing',
    keepSignature: true,
    stale: false,
    reload: false,
  });
  assert.deepEqual(signRefusal(failure(422, 'invalid_input')), {
    mode: 'editing',
    keepSignature: false,
    stale: false,
    reload: false,
  });
  for (const value of [failure(500, 'internal'), new Error('network')])
    assert.deepEqual(signRefusal(value), {
      mode: 'uncertain',
      keepSignature: true,
      stale: false,
      reload: false,
    });
});
